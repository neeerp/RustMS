#include "../Character/QuestResult.h"
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
		using ms::quest_result::ActItem;
		using ms::quest_result::inventory_full;

		require(inventory_full({ { 2000000, 5, -1, 2 } }, 0, 0, false) == "Use item inventory is full.", "one inventory");
		require(inventory_full({ { 2000000, 5, -1, 2 }, { 2000001, 1, -1, 2 }, { 4000000, 1, -1, 2 } }, 0, 0, false)
				== "Use orEtc item inventory is full.", "each inventory once, joined as v83 does");
		require(inventory_full({ { 1302000, 1, -1, 2 }, { 3010000, 1, -1, 2 } }, -100, 0, false)
				== "Either you don't have enough Mesos or Eqp orSetup item inventory is full.", "mesos taken");
		require(inventory_full({ { 4000000, -3, -1, 2 }, { 2000000, 0, -1, 2 } }, 0, 0, false) == " item inventory is full.",
				"items taken don't count");

		std::vector<ActItem> warrior_only = { { 1302000, 1, 1 << 1, 2 } };
		require(inventory_full(warrior_only, 0, 110, false) == "Eqp item inventory is full.", "the character's job");
		require(inventory_full(warrior_only, 0, 200, false) == " item inventory is full.", "another job");
		require(inventory_full(warrior_only, 0, 900, false) == "Eqp item inventory is full.", "GMs get every job's");

		std::vector<ActItem> dress = { { 1051000, 1, -1, 1 } };
		require(inventory_full(dress, 0, 0, true) == "Eqp item inventory is full.", "the character's gender");
		require(inventory_full(dress, 0, 0, false) == " item inventory is full.", "the other gender");

		std::cout << "Quest result passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
