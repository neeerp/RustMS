#include "../Gameplay/Physics/Physics.h"
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

// A tiny NX fixture: a flat floor from -1000 to 1000 at y = 100, above a wider
// one at y = 500 that keeps the map walls clear of the first floor's ends. It
// exercises the production collision/physics code without graphics, server
// processes, or game assets.
std::filesystem::path flat_map()
{
	const int64_t footholds[][6] = {{0, 0, -1000, 1000, 100, 100}, {0, 0, -2000, 2000, 500, 500}};
	const size_t count = std::size(footholds);
	std::vector<std::string> names {"", "0", "next", "prev", "x1", "x2", "y1", "y2"};
	for (size_t i = 0; i < count; ++i) names.push_back(std::to_string(i + 1));
	const size_t nodes = 3 + 7 * count;
	const size_t strings = 64 + nodes * 20;
	std::vector<uint8_t> bytes(strings + names.size() * 8);
	auto put = [&](size_t offset, uint64_t value, size_t count)
	{
		for (size_t i = 0; i < count; ++i) bytes[offset + i] = value >> (8 * i);
	};
	put(0, 0x34474b50, 4); put(4, nodes, 4); put(8, 64, 8);
	put(16, names.size(), 4); put(20, strings, 8);
	auto node = [&](size_t id, uint32_t name, uint32_t children, uint16_t count, uint16_t type, int64_t data)
	{
		size_t offset = 64 + id * 20;
		put(offset, name, 4); put(offset + 4, children, 4); put(offset + 8, count, 2);
		put(offset + 10, type, 2); put(offset + 12, data, 8);
	};
	node(0, 0, 1, 1, 0, 0); node(1, 1, 2, 1, 0, 0); node(2, 1, 3, count, 0, 0);
	for (size_t f = 0; f < count; ++f)
	{
		size_t values = 3 + count + 6 * f;
		node(3 + f, 8 + f, values, 6, 0, 0);
		for (size_t i = 0; i < 6; ++i) node(values + i, i + 2, 0, 0, 1, footholds[f][i]);
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

int main()
{
	std::filesystem::path path;
	try
	{
		path = flat_map();
		{
			nl::file map(path.string());
			ms::Physics physics(map.root());
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
			// A knocked back mob sets its own ground speed; friction must not
			// eat into it, even below the usual stopping threshold.
			ms::PhysicsObject mob;
			mob.set_x(0); mob.set_y(100);
			physics.move_object(mob);
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
				mob.set_x(start); mob.set_y(100);
				mob.set_flag(ms::PhysicsObject::Flag::TURNATEDGES);
				mob.set_flag(ms::PhysicsObject::Flag::WALKONLY);
				physics.move_object(mob);
				require(mob.onground && mob.fhid != 0, "walker fixture did not find its floor");
				for (int tick = 0; mob.is_flag_set(ms::PhysicsObject::Flag::TURNATEDGES); ++tick)
				{
					require(tick < 1000, "walker never reached the edge");
					mob.hforce = direction * .1;
					physics.move_object(mob);
				}
				require(mob.crnt_x() == direction * 980 && mob.onground, "walker did not stop 20 px from the edge");
			}
		}
		std::filesystem::remove(path);
		std::cout << "Jump momentum, contact transitions and walker edges passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		if (!path.empty()) std::filesystem::remove(path);
		std::cerr << error.what() << '\n';
		return 1;
	}
}
