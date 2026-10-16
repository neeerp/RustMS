#include "../Gameplay/Physics/Physics.h"
#include "../Gameplay/MapleMap/MapInfo.h"
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

struct TestFoothold
{
	int64_t group, id, x1, y1, x2, y2, prev, next;
};

// A tiny NX fixture holding the given footholds in layer 0. It exercises
// the production collision/physics code without graphics, server
// processes, or game assets.
std::filesystem::path foothold_map(std::vector<TestFoothold> footholds)
{
	// nlnx looks children up by name, so siblings are sorted by name.
	auto sorted = [](const TestFoothold& a, const TestFoothold& b)
	{
		return std::make_pair(std::to_string(a.group), std::to_string(a.id)) <
			std::make_pair(std::to_string(b.group), std::to_string(b.id));
	};
	std::sort(footholds.begin(), footholds.end(), sorted);
	std::vector<std::string> names {"", "0", "next", "prev", "x1", "x2", "y1", "y2"};
	auto name = [&](const std::string& text)
	{
		auto found = std::find(names.begin(), names.end(), text);
		if (found != names.end()) return static_cast<uint32_t>(found - names.begin());
		names.push_back(text);
		return static_cast<uint32_t>(names.size() - 1);
	};
	std::vector<int64_t> groups;
	for (const TestFoothold& fh : footholds)
	{
		if (groups.empty() || groups.back() != fh.group) groups.push_back(fh.group);
		name(std::to_string(fh.group)); name(std::to_string(fh.id));
	}
	size_t count = 2 + groups.size() + 7 * footholds.size();
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
	node(0, 0, 1, 1, 0, 0); node(1, 1, 2, groups.size(), 0, 0);
	size_t first = 2 + groups.size();
	for (size_t g = 0; g < groups.size(); ++g)
	{
		size_t members = std::count_if(footholds.begin(), footholds.end(),
			[&](const TestFoothold& fh) { return fh.group == groups[g]; });
		size_t before = std::count_if(footholds.begin(), footholds.end(),
			[&](const TestFoothold& fh) { return std::to_string(fh.group) < std::to_string(groups[g]); });
		node(2 + g, name(std::to_string(groups[g])), first + before, members, 0, 0);
	}
	for (size_t i = 0; i < footholds.size(); ++i)
	{
		const TestFoothold& fh = footholds[i];
		size_t leaves = first + footholds.size() + 6 * i;
		node(first + i, name(std::to_string(fh.id)), leaves, 6, 0, 0);
		const int64_t values[] = {fh.next, fh.prev, fh.x1, fh.x2, fh.y1, fh.y2};
		for (size_t j = 0; j < 6; ++j) node(leaves + j, 2 + j, 0, 0, 1, values[j]);
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
	require(bool(file), "could not write foothold fixture");
	return path;
}

// Flat footholds from x -1000 to 1000 at the given heights, the last one
// from -2000 to 2000.
std::filesystem::path flat_map(const std::vector<int64_t>& floors)
{
	std::vector<TestFoothold> footholds;
	for (size_t i = 0; i < floors.size(); ++i)
	{
		const int64_t width = i + 1 == floors.size() ? 2000 : 1000;
		footholds.push_back({0, static_cast<int64_t>(i + 1), -width, floors[i], width, floors[i], 0, 0});
	}
	return foothold_map(footholds);
}

// One v83 tick is 8 ms, so 1 pixel per tick is 125 pixels per second.
const double TICKS_PER_SECOND = 125.0;

// A map's info node, separate from its footholds, with numeric properties.
std::filesystem::path map_info(std::vector<std::pair<std::string, int64_t>> properties)
{
	std::sort(properties.begin(), properties.end());
	std::vector<std::string> names {"", "info"};
	for (const auto& property : properties) names.push_back(property.first);
	size_t nodes = 2 + properties.size();
	size_t strings = 64 + nodes * 20;
	std::vector<uint8_t> bytes(strings + names.size() * 8);
	auto put = [&](size_t offset, uint64_t value, size_t count)
	{
		for (size_t i = 0; i < count; ++i) bytes[offset + i] = value >> (8 * i);
	};
	auto node = [&](size_t id, size_t name, size_t first, size_t count, int type, int64_t value)
	{
		size_t offset = 64 + id * 20;
		put(offset, name, 4); put(offset + 4, first, 4); put(offset + 8, count, 2);
		put(offset + 10, type, 2); put(offset + 12, value, 8);
	};
	put(0, 0x34474b50, 4); put(4, nodes, 4); put(8, 64, 8);
	put(16, names.size(), 4); put(20, strings, 8);
	node(0, 0, 1, 1, 0, 0); node(1, 1, 2, properties.size(), 0, 0);
	for (size_t i = 0; i < properties.size(); ++i) node(2 + i, 2 + i, 0, 0, 1, properties[i].second);
	for (size_t i = 0; i < names.size(); ++i)
	{
		put(strings + i * 8, bytes.size(), 8);
		bytes.push_back(names[i].size()); bytes.push_back(0);
		bytes.insert(bytes.end(), names[i].begin(), names[i].end());
	}
	auto path = std::filesystem::temp_directory_path() /
		("rustms-map-info-" + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".nx");
	std::ofstream file(path, std::ios::binary);
	file.write(reinterpret_cast<const char*>(bytes.data()), bytes.size());
	require(bool(file), "could not write map-info fixture");
	return path;
}

// v83 uses zero as "no restriction" for movement, but as an actual camera edge.
void view_limits(nl::node footholds)
{
	for (bool zero : {false, true})
	{
		std::vector<std::pair<std::string, int64_t>> values {{"VRLimit", 1}};
		if (zero)
			for (const char* key : {"VRLeft", "VRRight", "VRTop", "VRBottom"}) values.emplace_back(key, 0);
		auto path = map_info(values);
		{
			nl::file info(path.string());
			ms::Physics physics(footholds, info.root()["info"]);
			auto walls = physics.get_fht().get_walls();
			auto borders = physics.get_fht().get_borders();
			require(walls.first() == -1970 && walls.second() == 1970 && borders.first() == -200,
				"zero or missing movement edges narrowed the physical bounds");
			ms::MapInfo view(info.root(), walls, borders);
			require(view.get_walls().first() == (zero ? 0 : -1990) && view.get_walls().second() == (zero ? 0 : 1990),
				"zero camera edges were treated as missing");
			require(view.get_borders().first() == (zero ? 0 : -260) && view.get_borders().second() == (zero ? 0 : 1410),
				"vertical camera edges used the wrong fallback");
			ms::PhysicsObject flier;
			flier.set_y(2000);
			physics.get_fht().limit_flight(flier);
			require(flier.crnt_y() == 1309, "a zero VRBottom narrowed the flying space");
		}
		std::filesystem::remove(path);
	}
	for (bool limit : {false, true})
	{
		auto path = map_info({{"VRLimit", limit}, {"VRLeft", -500}, {"VRRight", 600}, {"VRTop", -100}, {"VRBottom", 900}});
		{
			nl::file info(path.string());
			ms::Physics physics(footholds, info.root()["info"]);
			auto walls = physics.get_fht().get_walls();
			auto borders = physics.get_fht().get_borders();
			require(walls.first() == (limit ? -480 : -1970) && walls.second() == (limit ? 580 : 1970) &&
				borders.first() == (limit ? -35 : -200), "nonzero movement edges ignored VRLimit");
			ms::MapInfo view(info.root(), walls, borders);
			require(view.get_walls().first() == -500 && view.get_walls().second() == 600 &&
				view.get_borders().first() == -100 && view.get_borders().second() == 900,
				"camera edges incorrectly depended on VRLimit");
			ms::PhysicsObject flier;
			flier.set_y(2000);
			physics.get_fht().limit_flight(flier);
			require(flier.crnt_y() == (limit ? 899 : 1309), "nonzero VRBottom did not limit the flying space");
		}
		std::filesystem::remove(path);
	}
}

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

// A mob's hit knocks the player back and up at 270 px/s, about 18 px high
// and 73 px away.
void knockback(const ms::Physics& physics)
{
	for (double direction : {-1.0, 1.0})
	{
		ms::PhysicsObject player = grounded_shoe(physics);
		ms::Physics::knockback(player, direction * 270.0, -270.0);
		double peak = player.crnt_y();
		int ticks = 0;
		do
		{
			physics.move_object(player);
			require(++ticks == 1 ? !player.onground && player.vspeed < 0.0 : true, "a knockback did not lift off on its first tick");
			peak = std::min(peak, player.crnt_y());
			require(ticks < 100, "knockback never landed");
		} while (!player.onground);
		require(std::abs(100 - peak - 18.2) < 0.6, "knockback height differs from v83");
		require(std::abs(player.crnt_x() - direction * 73.0) < 1.5, "knockback distance differs from v83");
		require(!player.impact, "a knockback stayed queued after landing");
	}

	// Walking into the hit softens it, walking away doesn't add to it, and
	// the result is a whole number of pixels per second.
	for (int8_t walk : {1, -1})
	{
		ms::PhysicsObject player = grounded_shoe(physics);
		for (int tick = 0; tick < 3; ++tick)
		{
			player.walkdir = walk;
			physics.move_object(player);
		}
		require(std::abs(player.hspeed * TICKS_PER_SECOND - walk * 33.6) < 1e-9, "three walking ticks did not reach 33.6 px/s");
		ms::Physics::knockback(player, -270.0, -270.0);
		ms::Physics::knockback(player, -270.0, -270.0);
		physics.move_object(player);
		double expected = walk > 0 ? -236.0 : -270.0;
		require(std::abs(player.hspeed * TICKS_PER_SECOND - expected) < 0.01, "a knockback did not cap at its own speed");
	}
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

// A flying shoe at flySpeed 1 pushes at 1200 px/s^2 up to 200 px/s.
void flying(const ms::Physics& physics)
{
	auto flier = [&](double y)
	{
		ms::PhysicsObject mob;
		mob.shoe = true;
		mob.flyspeed = 1.0;
		mob.onground = false;
		mob.set_x(0); mob.set_y(y);
		return mob;
	};
	auto speed = [](double perTick) { return perTick * TICKS_PER_SECOND; };

	// Steering sideways while gliding: the glide settles on sinking at 60
	// px/s, picking up at half the push.
	ms::PhysicsObject mob = flier(-150);
	mob.walkdir = 1;
	mob.flydir = -1;
	physics.move_object(mob);
	require(std::abs(speed(mob.hspeed) - 9.6) < 1e-9, "a flier did not push at 1200 px/s^2");
	require(std::abs(speed(mob.vspeed) - 4.8) < 1e-9, "a glide did not pick up at 600 px/s^2");
	for (int tick = 0; tick < 50; ++tick)
	{
		mob.walkdir = 1;
		physics.move_object(mob);
	}
	require(std::abs(speed(mob.hspeed) - 200.0) < 1e-9, "a flier did not top out at its flySpeed");
	require(std::abs(speed(mob.vspeed) - 60.0) < 1e-9, "a glide did not sink at 0.3 of flySpeed");

	// A glide brakes a faster fall at the full push.
	mob.vspeed = 100.0 / TICKS_PER_SECOND;
	physics.move_object(mob);
	require(std::abs(speed(mob.vspeed) - 90.4) < 1e-9, "a glide did not brake a fall at 1200 px/s^2");

	// Not steering, it slows by floatDrag1 / mass, and twice over while
	// faster than its flySpeed.
	mob = flier(-150);
	mob.flydir = -1;
	mob.hspeed = 300.0 / TICKS_PER_SECOND;
	physics.move_object(mob);
	require(std::abs(speed(mob.hspeed) - 284.0) < 1e-9, "a fast flier did not slow at twice 1000 px/s^2");
	mob.walkdir = 1;
	physics.move_object(mob);
	require(std::abs(speed(mob.hspeed) - 276.0) < 1e-9, "a fast flier steering on did not slow at 1000 px/s^2");

	// Diving speeds up to its flySpeed, through every floor, and stops at
	// the bottom of the map's physical space, 10 px below its lowest floor.
	mob = flier(-100);
	double fastest = 0.0;
	for (int tick = 0; tick < 1000; ++tick)
	{
		physics.move_object(mob);
		fastest = std::max(fastest, speed(mob.vspeed));
		require(!mob.onground, "a flier landed");
	}
	require(mob.crnt_y() == 1309 && std::abs(fastest - 200.0) < 1e-9, "a dive did not keep on at flySpeed to the map's bottom");
	require(std::abs(speed(mob.vspeed) - 200.0) < 1e-9, "the map's bottom slowed a flier");

	// The sides of the space are 30 px in from the platforms.
	mob = flier(-150);
	mob.set_x(-1960);
	for (int tick = 0; tick < 20; ++tick)
	{
		mob.walkdir = -1;
		physics.move_object(mob);
	}
	require(mob.crnt_x() == -1970, "a flier left the map's side");

	// Its jump beats up at its full flySpeed, keeping its sideways speed.
	mob = flier(-150);
	mob.hspeed = 50.0 / TICKS_PER_SECOND;
	ms::Physics::fly_jump(mob);
	require(std::abs(speed(mob.vspeed) + 200.0) < 1e-9 && std::abs(speed(mob.hspeed) - 50.0) < 1e-9, "a flier's beat was wrong");
}

// A 100 px wall stops a walking player whichever way the map draws it:
// going up along the chain (y1 > y2) or going down (y1 < y2).
void walls(const ms::Physics& physics)
{
	struct Case { double x; int8_t direction; uint16_t fhid; double stop; };
	for (Case c : {Case {-100, 1, 1, 0}, Case {2100, -1, 6, 2000}})
	{
		ms::PhysicsObject player;
		player.shoe = true;
		player.set_x(c.x); player.set_y(0);
		physics.move_object(player);
		require(player.onground && player.fhid == c.fhid, "wall fixture did not find its floor");
		for (int tick = 0; tick < 200; ++tick)
		{
			player.walkdir = c.direction;
			physics.move_object(player);
		}
		require(player.onground && player.fhid == c.fhid && player.crnt_x() == c.stop, "a player walked through a wall");
	}
}

// Group 1 is a floor at y 0 from x -500 to 500, a 100 px wall up at 500
// and a floor on from there to 800, with a ledge from -300 to -100 at y
// -150. Group 2 is a separate floor from 1000 to 1400, and group 3 keeps
// the edges of the map out of the way.
std::filesystem::path jumper_map()
{
	return foothold_map({
		{3, 7, -2000, 500, -1500, 500, 0, 0},
		{3, 8, 1500, 500, 2000, 500, 0, 0},
		{1, 1, -500, 0, 0, 0, 0, 2},
		{1, 2, 0, 0, 500, 0, 1, 3},
		{1, 3, 500, 0, 500, -100, 2, 4},
		{1, 4, 500, -100, 800, -100, 3, 0},
		{1, 5, -300, -150, -100, -150, 0, 0},
		{2, 6, 1000, 0, 1400, 0, 0, 0},
	});
}

ms::PhysicsObject jumper_at(const ms::Physics& physics, double x, double y, uint16_t homegroup)
{
	ms::PhysicsObject mob;
	mob.shoe = true;
	mob.set_x(x); mob.set_y(y);
	mob.set_flag(ms::PhysicsObject::Flag::TURNATEDGES);
	mob.set_flag(ms::PhysicsObject::Flag::JUMPER);
	mob.homegroup = homegroup;
	physics.move_object(mob);
	require(mob.onground && mob.fhid != 0, "jumper fixture did not find its floor");
	return mob;
}

// Walks a jumper until it asks to jump, turns, or leaves the ground.
void walk_jumper(const ms::Physics& physics, ms::PhysicsObject& mob, int8_t direction)
{
	for (int tick = 0; !mob.jumprequest && mob.onground && mob.is_flag_set(ms::PhysicsObject::Flag::TURNATEDGES); ++tick)
	{
		require(tick < 1000, "jumper never got anywhere");
		mob.walkdir = direction;
		physics.move_object(mob);
	}
}

// v83 (FUN_007f8d64) keeps a jumping mob within its groups' reach and has
// it stop to jump at loose ends and rising walls inside it.
void jumping_mobs(const ms::Physics& physics)
{
	const ms::FootholdTree& fht = physics.get_fht();
	require(fht.get_group_reach(1).first() == -500 && fht.get_group_reach(1).second() == 800, "group 1 reach was wrong");
	require(fht.get_group_reach(9).first() > fht.get_group_reach(9).second(), "a missing group reached somewhere");
	require(fht.get_group_below({-200, -150}) == 1 && fht.get_group_below({1200, -50}) == 2, "the group below was wrong");

	// The reach is shrunk by 20 px a side, and leaving it turns the mob.
	ms::PhysicsObject mob = jumper_at(physics, -400, 0, 1);
	walk_jumper(physics, mob, -1);
	require(mob.crnt_x() == -480 && mob.onground && !mob.jumprequest, "jumper did not turn 20 px inside its reach");
	mob = jumper_at(physics, 700, -100, 1);
	walk_jumper(physics, mob, 1);
	require(mob.crnt_x() == 780 && !mob.jumprequest, "jumper did not turn 20 px inside its reach");

	// Its home group widens the reach, so it gets to the loose end and
	// stops there to jump.
	mob = jumper_at(physics, 700, -100, 2);
	walk_jumper(physics, mob, 1);
	require(mob.crnt_x() == 800 && mob.onground && mob.jumprequest, "jumper did not stop to jump at a loose end");
	require(mob.is_flag_set(ms::PhysicsObject::Flag::TURNATEDGES), "jumper turned at a loose end");

	// At a rising wall it stops to jump; told not to, it only stops.
	for (bool edgejump : {true, false})
	{
		mob = jumper_at(physics, 400, 0, 1);
		mob.edgejump = edgejump;
		for (int tick = 0; tick < 300; ++tick)
		{
			mob.walkdir = 1;
			physics.move_object(mob);
			if (mob.jumprequest) break;
		}
		require(mob.crnt_x() == 500 && mob.onground && mob.jumprequest == edgejump, "jumper did not stop at the wall");
		require(mob.is_flag_set(ms::PhysicsObject::Flag::TURNATEDGES), "jumper turned at a wall");
	}

	// At a loose end of the ledge it stops to jump, or walks off.
	mob = jumper_at(physics, -200, -150, 1);
	walk_jumper(physics, mob, 1);
	require(mob.crnt_x() == -100 && mob.jumprequest, "jumper did not stop to jump at the ledge's end");
	mob = jumper_at(physics, -200, -150, 1);
	mob.edgejump = false;
	walk_jumper(physics, mob, 1);
	require(!mob.onground && mob.crnt_x() > -100, "jumper did not walk off the ledge");

	// Down the wall is a step it walks down, unless told to jump it.
	mob = jumper_at(physics, 600, -100, 1);
	walk_jumper(physics, mob, -1);
	require(!mob.onground && mob.crnt_x() < 500, "jumper did not walk down the step");
	mob = jumper_at(physics, 600, -100, 1);
	mob.stepdown = false;
	walk_jumper(physics, mob, -1);
	require(mob.crnt_x() == 500 && mob.jumprequest, "jumper did not stop to jump down the step");

	// The AI sees a rising wall 40 px ahead, and whether it is against it.
	using WallAhead = ms::FootholdTree::WallAhead;
	require(fht.get_wall_ahead(jumper_at(physics, 459, 0, 1), 1) == WallAhead::NONE, "a wall 41 px away was seen");
	require(fht.get_wall_ahead(jumper_at(physics, 470, 0, 1), 1) == WallAhead::NEAR, "a wall 30 px away was missed");
	require(fht.get_wall_ahead(jumper_at(physics, 470, 0, 1), -1) == WallAhead::NONE, "a wall behind was seen");
	require(fht.get_wall_ahead(jumper_at(physics, -10, 0, 1), 1) == WallAhead::NONE, "a floor ahead was taken for a wall");
	require(fht.get_wall_ahead(jumper_at(physics, 520, -100, 1), -1) == WallAhead::NONE, "a step down was taken for a wall");
	require(fht.get_wall_ahead(jumper_at(physics, -110, -150, 1), 1) == WallAhead::NONE, "a loose end was taken for a wall");
	mob = jumper_at(physics, 400, 0, 1);
	walk_jumper(physics, mob, 1);
	require(fht.get_wall_ahead(mob, 1) == WallAhead::AGAINST, "a mob at the wall was not against it");
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
			view_limits(map.root());
			// v83 keeps objects 30 px inside the outermost footholds.
			auto walls = physics.get_fht().get_walls();
			require(walls.first() == -1970 && walls.second() == 1970, "map walls are not 30 px inside the footholds");
			legacy_jumps_keep_momentum(physics);
			walking(physics);
			mob_walking(physics);
			standing_jump(physics);
			running_jump(physics);
			falling(physics);
			flying(physics);
			jumping_down(physics);
			knockback(physics);
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
			std::filesystem::path jumpers = jumper_map();
			try
			{
				nl::file map(jumpers.string());
				ms::Physics physics(map.root());
				jumping_mobs(physics);
			}
			catch (...)
			{
				std::filesystem::remove(jumpers);
				throw;
			}
			std::filesystem::remove(jumpers);
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
		// Ellinia's in00 stands on a foothold rising 3 px over 105, over a
		// floor 165 px down. Stopping on it must not drop through it.
		path = foothold_map({{1, 1, 757, -2280, 862, -2283, 0, 0}, {2, 2, 600, -2116, 1000, -2116, 0, 0}});
		{
			nl::file map(path.string());
			ms::Physics physics(map.root());
			for (int8_t direction : {-1, 1})
			{
				ms::PhysicsObject player;
				player.shoe = true;
				player.set_x(810); player.set_y(physics.get_y_below({810, -2300}).y());
				for (int tick = 0; tick < 5; ++tick)
					physics.move_object(player);
				for (int tick = 0; tick < 60; ++tick)
				{
					player.walkdir = tick < 10 ? direction : 0;
					physics.move_object(player);
					require(player.onground && player.fhid == 1, "stopping on a slope dropped through it");
				}
				require(player.hspeed == 0.0, "the walker did not stop");
			}
		}
		std::filesystem::remove(path);
		path = foothold_map({
			{1, 1, -500, 0, 0, 0, 0, 2}, {1, 2, 0, 0, 0, -100, 1, 3}, {1, 3, 0, -100, 500, -100, 2, 0},
			{2, 4, 1500, -100, 2000, -100, 0, 5}, {2, 5, 2000, -100, 2000, 0, 4, 6}, {2, 6, 2000, 0, 2500, 0, 5, 0},
			{3, 7, -3000, 600, 4000, 600, 0, 0}});
		{
			nl::file map(path.string());
			ms::Physics physics(map.root());
			walls(physics);
			// Fliers pick their idle waypoints from a group's footholds.
			const ms::FootholdTree& fht = physics.get_fht();
			require(fht.get_groups() == std::vector<uint16_t>{1, 2, 3}, "foothold groups were not listed in load order");
			require(fht.get_group_footholds(2) == std::vector<uint16_t>{4, 5, 6}, "a group's footholds were not listed");
			require(fht.get_group_footholds(9).empty(), "a missing group had footholds");
			require(fht.get_fh(2).length() == 100.0 && fht.get_fh(7).length() == 7000.0, "foothold lengths were wrong");
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
