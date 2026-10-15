#include "../Gameplay/Physics/Physics.h"
#include <algorithm>
#include <nlnx/file.hpp>
#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

// A tiny NX fixture of flat footholds from x -1000 to 1000 at the given
// heights. It exercises the production collision/physics code without
// graphics, server processes, or game assets.
std::filesystem::path flat_map(const std::vector<int64_t>& floors)
{
	std::vector<std::string> names {"", "0", "next", "prev", "x1", "x2", "y1", "y2"};
	for (size_t i = 0; i < floors.size(); ++i) names.push_back(std::to_string(i + 1));
	size_t count = 3 + 7 * floors.size();
	size_t strings = 64 + count * 20;
	std::vector<uint8_t> bytes(strings + names.size() * 8);
	auto put = [&](size_t offset, uint64_t value, size_t count)
	{
		for (size_t i = 0; i < count; ++i) bytes[offset + i] = value >> (8 * i);
	};
	put(0, 0x34474b50, 4); put(4, count, 4); put(8, 64, 8);
	put(16, names.size(), 4); put(20, strings, 8);
	auto node = [&](size_t id, uint32_t name, uint32_t children, uint16_t count, uint16_t type, int64_t data)
	{
		size_t offset = 64 + id * 20;
		put(offset, name, 4); put(offset + 4, children, 4); put(offset + 8, count, 2);
		put(offset + 10, type, 2); put(offset + 12, data, 8);
	};
	node(0, 0, 1, 1, 0, 0); node(1, 1, 2, 1, 0, 0);
	node(2, 1, 3, floors.size(), 0, 0);
	for (size_t i = 0; i < floors.size(); ++i)
	{
		size_t first = 3 + floors.size() + 6 * i;
		node(3 + i, 8 + i, first, 6, 0, 0);
		const int64_t width = i + 1 == floors.size() ? 2000 : 1000;
		const int64_t values[] = {0, 0, -width, width, floors[i], floors[i]};
		for (size_t j = 0; j < 6; ++j) node(first + j, 2 + j, 0, 0, 1, values[j]);
	}
	for (size_t i = 0; i < names.size(); ++i)
	{
		put(strings + i * 8, bytes.size(), 8);
		bytes.push_back(names[i].size()); bytes.push_back(0);
		bytes.insert(bytes.end(), names[i].begin(), names[i].end());
	}
	auto path = std::filesystem::temp_directory_path() /
		("rustms-movement-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".nx");
	std::ofstream file(path, std::ios::binary);
	file.write(reinterpret_cast<const char*>(bytes.data()), bytes.size());
	require(bool(file), "could not write flat-map fixture");
	return path;
}

// One v83 tick is 8 ms, so 1 pixel per tick is 125 pixels per second.
const double TICKS_PER_SECOND = 125.0;

ms::PhysicsObject grounded_shoe(const ms::Physics& physics)
{
	ms::PhysicsObject player;
	player.shoe = true;
	player.set_x(0); player.set_y(100);
	physics.move_object(player);
	require(player.onground && player.fhid != 0, "fixture did not find its floor");
	return player;
}

// HeavenClient's own model, which flying mobs, NPCs and pets still use.
void legacy_jumps_keep_momentum(const ms::Physics& physics)
{
	for (double direction : {-1.0, 1.0})
	{
		ms::PhysicsObject player;
		player.set_x(0); player.set_y(100);
		physics.move_object(player);
		require(player.onground && player.fhid != 0, "fixture did not find its floor");
		player.hspeed = direction * 1.2;
		player.hforce = direction * .16;
		player.vforce = -4.5;
		physics.move_object(player);
		require(!player.onground && player.vspeed < 0.0, "takeoff must become airborne on the same tick");
		double momentum = player.hspeed;
		for (int tick = 0; tick < 100; ++tick)
		{
			// Releasing the direction key while airborne adds no force.
			player.hforce = 0.0;
			physics.move_object(player);
			require(std::abs(player.hspeed - momentum) < 1e-9, "jump lost horizontal momentum");
			if (player.vspeed == 0.0)
			{
				require(player.onground && player.crnt_y() == 100, "landing must become grounded on the collision tick");
				break;
			}
			require(!player.onground, "airborne motion was classified as grounded");
			require(tick < 99, "jump never landed");
		}
		// A jump requested immediately on landing must work without an
		// extra settling tick, and maintain the running horizontal speed.
		player.hforce = direction * .16;
		player.vforce = -4.5;
		physics.move_object(player);
		require(player.vspeed < 0.0 && !player.onground, "landing-time jump was lost");
		require(std::abs(player.hspeed - momentum) < 1e-9, "running restart lost momentum");
	}
}

// v83 walks at 1400 px/s^2 up to 125 px/s and stops at 800 px/s^2.
void walking(const ms::Physics& physics)
{
	for (int8_t direction : {-1, 1})
	{
		ms::PhysicsObject player = grounded_shoe(physics);
		int ticks = 0;
		while (direction * player.hspeed < ms::Physics::walk_speed(1.0))
		{
			player.walkdir = direction;
			physics.move_object(player);
			require(++ticks <= 12, "walking was slower to reach top speed than v83");
		}
		require(ticks == 12, "walking reached top speed faster than v83");
		for (int tick = 0; tick < 50; ++tick)
		{
			player.walkdir = direction;
			physics.move_object(player);
			require(std::abs(player.hspeed * TICKS_PER_SECOND - direction * 125.0) < 1e-9, "walking left the v83 top speed");
		}

		double from = player.crnt_x();
		ticks = 0;
		while (player.hspeed != 0.0)
		{
			physics.move_object(player);
			require(++ticks <= 20, "letting go slid further than v83");
		}
		double slide = direction * (player.crnt_x() - from);
		require(ticks == 20 && slide > 9.0 && slide < 10.5, "letting go stopped at a different distance from v83");

		// The SPEED stat scales the top speed but not the acceleration.
		player.walkspeed = 1.4;
		for (int tick = 0; tick < 50; ++tick)
		{
			player.walkdir = direction;
			physics.move_object(player);
		}
		require(std::abs(player.hspeed * TICKS_PER_SECOND - direction * 175.0) < 1e-9, "140% speed did not walk at 175 px/s");
	}
}

// A snail (info/speed -65, info/fs 10) walks at 35% of 125 px/s. Its fs
// multiplies the shoe's acceleration and drag tenfold, so it starts and
// stops almost at once.
void mob_walking(const ms::Physics& physics)
{
	for (int8_t direction : {-1, 1})
	{
		ms::PhysicsObject snail = grounded_shoe(physics);
		snail.walkspeed = 0.35;
		snail.walkforce = 10.0;
		snail.walkdrag = 10.0;
		snail.walkdir = direction;
		physics.move_object(snail);
		require(std::abs(snail.hspeed * TICKS_PER_SECOND - direction * 43.75) < 1e-9, "a snail took more than a tick to reach top speed");

		int ticks = 0;
		while (snail.hspeed != 0.0)
		{
			physics.move_object(snail);
			require(++ticks <= 4, "a snail slid further than v83");
		}
		require(ticks == 4, "a snail stopped sooner than v83");
	}

	// fs below 1 is clamped to 0.05 and halves the drag, but not the push.
	ms::PhysicsObject slippery = grounded_shoe(physics);
	slippery.walkforce = 0.5;
	slippery.walkdrag = 0.5;
	slippery.walkdir = 1;
	physics.move_object(slippery);
	require(std::abs(slippery.hspeed * TICKS_PER_SECOND - 700.0 * 0.008) < 1e-9, "fs 0.5 did not halve the push");
	double speed = slippery.hspeed;
	physics.move_object(slippery);
	require(std::abs((speed - slippery.hspeed) * TICKS_PER_SECOND - 200.0 * 0.008) < 1e-9, "fs 0.5 did not quarter the drag");
}

// A standing jump rises 555^2 / (2 * 2000) = 77 px and lands after 0.555 s.
void standing_jump(const ms::Physics& physics)
{
	ms::PhysicsObject player = grounded_shoe(physics);
	ms::Physics::jump(player, ms::Physics::jump_speed(1.0), 0);
	require(player.hspeed == 0.0, "a standing jump gained sideways speed");
	double peak = player.crnt_y();
	int ticks = 0;
	do
	{
		physics.move_object(player);
		require(++ticks == 1 ? !player.onground && player.vspeed < 0.0 : true, "takeoff must become airborne on the same tick");
		peak = std::min(peak, player.crnt_y());
		require(ticks < 100, "jump never landed");
	} while (!player.onground);
	require(std::abs(100 - peak - 77.0) < 0.6, "jump height differs from v83");
	require(ticks >= 69 && ticks <= 70, "jump airtime differs from v83");
	require(player.crnt_y() == 100 && player.vspeed == 0.0, "landing must become grounded on the collision tick");
}

void running_jump(const ms::Physics& physics)
{
	for (int8_t direction : {-1, 1})
	{
		// Jumping from a standstill while holding a direction starts at 80%
		// of the top walking speed.
		ms::PhysicsObject player = grounded_shoe(physics);
		ms::Physics::jump(player, ms::Physics::jump_speed(1.0), direction);
		require(std::abs(player.hspeed * TICKS_PER_SECOND - direction * 100.0) < 1e-9, "a standing jump with a direction did not kick to 100 px/s");

		player = grounded_shoe(physics);
		for (int tick = 0; tick < 30; ++tick)
		{
			player.walkdir = direction;
			physics.move_object(player);
		}
		double momentum = player.hspeed;
		ms::Physics::jump(player, ms::Physics::jump_speed(1.0), direction);
		require(player.hspeed == momentum, "a running jump changed the walking speed");
		for (int tick = 0; tick < 100 && (tick == 0 || !player.onground); ++tick)
		{
			// Without input the air barely slows the jump, at 1 px/s^2.
			physics.move_object(player);
			require(direction * player.hspeed <= direction * momentum && direction * (momentum - player.hspeed) < 0.6 / TICKS_PER_SECOND, "jump lost horizontal momentum");
		}
		require(player.onground, "jump never landed");

		// A jump requested immediately on landing must work without an
		// extra settling tick.
		ms::Physics::jump(player, ms::Physics::jump_speed(1.0), direction);
		physics.move_object(player);
		require(player.vspeed < 0.0 && !player.onground, "landing-time jump was lost");

		// Holding the other way pulls the jump back at 200 px/s^2.
		double before = player.hspeed;
		for (int tick = 0; tick < 25; ++tick)
		{
			player.walkdir = -direction;
			physics.move_object(player);
		}
		require(std::abs(direction * (before - player.hspeed) * TICKS_PER_SECOND - 40.0) < 1e-6, "steering against a jump differs from v83");
	}
}

// A down-jump hops at 555 / sqrt(8) = 196 px/s, about 9.6 px, falls through
// its own floor, and lands on the next one.
void jumping_down(const ms::Physics& physics)
{
	ms::PhysicsObject player = grounded_shoe(physics);
	player.set_flag(ms::PhysicsObject::Flag::CHECKBELOW);
	physics.move_object(player);
	require(player.enablejd, "a floor 150 px down did not allow a down-jump");

	ms::Physics::jump_down(player);
	require(player.hspeed == 0.0 && std::abs(player.vspeed * TICKS_PER_SECOND + 196.22) < 0.01, "the down-jump hop differs from v83");
	double peak = player.crnt_y();
	for (int tick = 0; tick < 200 && !player.onground; ++tick)
	{
		physics.move_object(player);
		peak = std::min(peak, player.crnt_y());
	}
	require(std::abs(100 - peak - 9.6) < 0.6, "the down-jump hop height differs from v83");
	require(player.onground && player.crnt_y() == 250 && player.fallstart == 0, "a down-jump did not land on the floor below");

	// With nothing within 300 px, a floor 300 to 600 px down still counts.
	player.set_flag(ms::PhysicsObject::Flag::CHECKBELOW);
	physics.move_object(player);
	require(player.enablejd, "a floor 350 px down did not allow a down-jump");
	ms::Physics::jump_down(player);
	for (int tick = 0; tick < 200 && !player.onground; ++tick)
		physics.move_object(player);
	require(player.crnt_y() == 600, "the second down-jump did not land 350 px down");

	// A floor 700 px down is too far.
	player.set_flag(ms::PhysicsObject::Flag::CHECKBELOW);
	physics.move_object(player);
	require(!player.enablejd, "a floor 700 px down allowed a down-jump");
}

// Falls speed up at 2000 px/s^2 to at most 670 px/s.
void falling(const ms::Physics& physics)
{
	ms::PhysicsObject player;
	player.shoe = true;
	player.set_x(0); player.set_y(-150);
	double fastest = 0.0;
	for (int tick = 0; tick < 100 && (tick == 0 || !player.onground); ++tick)
	{
		physics.move_object(player);
		fastest = std::max(fastest, player.vspeed);
		require(player.vspeed * TICKS_PER_SECOND <= 670.0 + 1e-9, "falling passed the v83 fall speed");
	}
	require(player.onground && std::abs(fastest * TICKS_PER_SECOND - 670.0) < 1e-9, "a long fall did not reach the v83 fall speed");
}

int main()
{
	std::filesystem::path path;
	try
	{
		path = flat_map({100, 250, 600, 1300});
		{
			nl::file map(path.string());
			ms::Physics physics(map.root());
			legacy_jumps_keep_momentum(physics);
			walking(physics);
			mob_walking(physics);
			standing_jump(physics);
			running_jump(physics);
			falling(physics);
			jumping_down(physics);
			// A knocked back mob sets its own ground speed; friction must not
			// eat into it, even below the usual stopping threshold.
			ms::PhysicsObject mob = grounded_shoe(physics);
			mob.walkforce = 10.0;
			mob.walkdrag = 10.0;
			mob.set_flag(ms::PhysicsObject::Flag::NOFRICTION);
			for (double speed : {1.04, 0.05})
			{
				mob.hspeed = speed;
				double start = mob.crnt_x();
				physics.move_object(mob);
				require(mob.onground && mob.hspeed == speed, "ground friction slowed a NOFRICTION object");
				require(std::abs(mob.crnt_x() - start - speed) < 1e-9, "NOFRICTION object moved the wrong distance");
			}
		}
		{
			// A walking-only mob turns 20 px before the loose ends of the
			// floor, and is pulled back if it starts past that point.
			nl::file map(path.string());
			ms::Physics physics(map.root());
			for (double start : {900.0, 990.0, -900.0, -990.0})
			{
				double direction = start > 0.0 ? 1.0 : -1.0;
				ms::PhysicsObject mob;
				mob.shoe = true;
				mob.walkspeed = 0.35;
				mob.set_x(start); mob.set_y(100);
				mob.set_flag(ms::PhysicsObject::Flag::TURNATEDGES);
				mob.set_flag(ms::PhysicsObject::Flag::WALKONLY);
				physics.move_object(mob);
				require(mob.onground && mob.fhid != 0, "walker fixture did not find its floor");
				for (int tick = 0; mob.is_flag_set(ms::PhysicsObject::Flag::TURNATEDGES); ++tick)
				{
					require(tick < 1000, "walker never reached the edge");
					mob.walkdir = direction;
					physics.move_object(mob);
				}
				require(mob.crnt_x() == direction * 980 && mob.onground, "walker did not stop 20 px from the edge");
			}
		}
		{
			// A mob that enters on a foothold stands on it, even if its
			// position is a pixel below it, and stays there.
			nl::file map(path.string());
			ms::Physics physics(map.root());
			ms::PhysicsObject mob;
			mob.set_x(500); mob.set_y(101);
			physics.get_fht().place_on_fh(mob, 1);
			require(mob.crnt_y() == 100 && mob.onground && mob.fhid == 1, "spawn was not put on its foothold");
			physics.move_object(mob);
			require(mob.crnt_y() == 100 && mob.onground, "placed mob fell");
			mob.set_x(1200); mob.set_y(50);
			physics.get_fht().place_on_fh(mob, 1);
			require(mob.crnt_x() == 1000 && mob.crnt_y() == 100, "placement was not clamped to the foothold");
		}
		std::filesystem::remove(path);
		std::cout << "Movement physics passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		if (!path.empty()) std::filesystem::remove(path);
		std::cerr << error.what() << '\n';
		return 1;
	}
}
