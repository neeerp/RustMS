#include "../Gameplay/NpcScripts.h"
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
		using npc_scripts::active;
		using npc_scripts::Window;

		require(active({}, 20261023).empty(), "an NPC with no windows has no script");

		std::vector<Window> windows = { { "always", 19000101, 20790101 } };
		require(active(windows, 20261023) == "always", "the Npc.nx defaults cover every day");

		windows.push_back({ "event", 20261201, 20261231 });
		require(active(windows, 20261023) == "always", "a window not open yet is skipped");
		require(active(windows, 20261201) == "event", "the newest open window wins, from its first day");
		require(active(windows, 20261231) == "event", "the last day is included");
		require(active(windows, 20270101) == "always", "after it ends the older window is back");

		windows.push_back({ "", 20261225, 20261225 });
		require(active(windows, 20261225).empty(), "an open window without a script takes the script away");

		require(active({ { "old", 19000101, 20000101 } }, 20261023).empty(), "a closed window gives nothing");

		npc_scripts::received()[9000021].push_back({ "gaga", 0, 0x7fffffff });
		npc_scripts::received()[9000021].push_back({ "gaga2", 0, 0x7fffffff });
		require(npc_scripts::received().at(9000021).size() == 2, "each 0x107 entry is kept");
		require(active(npc_scripts::received().at(9000021), 20261023) == "gaga2", "the last one sent wins");
		require(!npc_scripts::received().count(9000020), "other NPCs are untouched");

		std::cout << "NPC scripts passed\n";
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	return 0;
}
