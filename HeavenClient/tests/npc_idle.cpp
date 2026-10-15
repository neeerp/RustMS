#include "../Gameplay/MapleMap/NpcIdle.h"
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
		using npc_idle::Choice;
		using npc_idle::choose;

		// say (2 lines) and eye (none), plus 3 info/speak lines.
		std::vector<int32_t> actions = { 2, 0 };
		Choice choice;

		require(choose(actions, 0, -1, 3, 0, 7, choice), "an NPC with actions and lines always does something");
		require(choice.action == 0 && choice.line == 1, "the first roll picks the action, the second its line");
		require(choose(actions, 0, -1, 3, 1, 7, choice), "an action without lines is still played");
		require(choice.action == 1 && choice.line == -1, "an action without lines says nothing");
		require(choose(actions, 0, -1, 3, 4, 0, choice), "rolls past the actions say an idle line");
		require(choice.action == -1 && choice.line == 2, "the idle line follows the actions");
		require(choose(actions, 0, -1, 3, 49, 0, choice), "the roll wraps around the choices");
		require(choice.action == -1 && choice.line == 2, "49 % 5 is the last idle line");

		require(choose({ 0, 0 }, 1, -1, 0, 1, 0, choice), "a special action is left out of the draw");
		require(choice.action == 0, "only the first (actions - specials) can be drawn");

		require(!choose({ 1, 0 }, 0, 1, 0, 1, 0, choice), "drawing the shop action does nothing");
		require(choose({ 1, 0 }, 0, 1, 0, 0, 0, choice) && choice.action == 0, "other actions still play");

		require(!choose({}, 0, -1, 0, 0, 0, choice), "an NPC with nothing to do stays still");
		require(choose({}, 0, -1, 2, 3, 0, choice) && choice.line == 1, "an NPC with only lines says them");

		std::cout << "NPC idle passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
