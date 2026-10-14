#include "../Gameplay/MapleMap/NpcReach.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms;
		using npc_reach::in_reach;
		using npc_reach::nearer;

		Point<int16_t> player(0, 200);

		require(in_reach(player, Point<int16_t>(120, 200)), "an NPC 120 px away on the same floor is in reach");
		require(in_reach(player, Point<int16_t>(-150, 200)), "reach is the same on both sides");
		require(!in_reach(player, Point<int16_t>(151, 200)), "an NPC further than 150 px is out of reach");
		require(in_reach(player, Point<int16_t>(40, 230)), "a small step in the floor is still in reach");
		require(!in_reach(player, Point<int16_t>(0, 120)), "an NPC on the platform above is out of reach");

		require(nearer(player, Point<int16_t>(-30, 200), Point<int16_t>(60, 200)), "the horizontally nearer NPC wins");
		require(!nearer(player, Point<int16_t>(60, 200), Point<int16_t>(-30, 200)), "the further NPC loses");
		require(nearer(player, Point<int16_t>(30, 205), Point<int16_t>(-30, 230)), "a tie goes to the NPC on the player's level");

		std::cout << "NPC reach passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
