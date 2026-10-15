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

// A tiny NX fixture with a single flat foothold. It exercises the production
// collision/physics code without graphics, server processes, or game assets.
std::filesystem::path flat_map()
{
	std::vector<std::string> names {"", "0", "1", "next", "prev", "x1", "x2", "y1", "y2"};
	std::vector<uint8_t> bytes(64 + 10 * 20 + names.size() * 8);
	auto put = [&](size_t offset, uint64_t value, size_t count)
	{
		for (size_t i = 0; i < count; ++i) bytes[offset + i] = value >> (8 * i);
	};
	put(0, 0x34474b50, 4); put(4, 10, 4); put(8, 64, 8);
	put(16, names.size(), 4); put(20, 264, 8);
	auto node = [&](size_t id, uint32_t name, uint32_t children, uint16_t count, uint16_t type, int64_t data)
	{
		size_t offset = 64 + id * 20;
		put(offset, name, 4); put(offset + 4, children, 4); put(offset + 8, count, 2);
		put(offset + 10, type, 2); put(offset + 12, data, 8);
	};
	node(0, 0, 1, 1, 0, 0); node(1, 1, 2, 1, 0, 0);
	node(2, 1, 3, 1, 0, 0); node(3, 2, 4, 6, 0, 0);
	const int64_t values[] = {0, 0, -1000, 1000, 100, 100};
	for (size_t i = 0; i < 6; ++i) node(i + 4, i + 3, 0, 0, 1, values[i]);
	for (size_t i = 0; i < names.size(); ++i)
	{
		put(264 + i * 8, bytes.size(), 8);
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
		std::cout << "Jump momentum, contact transitions and spawn placement passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		if (!path.empty()) std::filesystem::remove(path);
		std::cerr << error.what() << '\n';
		return 1;
	}
}
