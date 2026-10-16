#include "../Character/Inventory/ItemDrop.h"
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
		using namespace ms::item_drop;

		require(refused(5160000, true), "a cash item stays in the inventory");
		require(refused(4220000, false), "a Flowerpot stays in the inventory");
		require(!refused(4000000, false), "a Snail Shell can be dropped");

		require(asks(false, false) == 0, "an ordinary item drops without a question");
		require(asks(true, false) == 1, "Maria's Letter, a quest item, asks once");
		require(asks(false, true) == 1, "an untradable item asks once");
		require(asks(true, true) == 2, "Nine Spirit's Egg, quest and untradable, asks twice");

		require(asks_count(4000000, 10), "a stack of Snail Shells asks how many");
		require(asks_count(2000000, 20), "a stack of potions asks how many");
		require(asks_count(3010000, 2), "a stack of chairs asks how many");
		require(!asks_count(4000000, 1), "a single item drops as one");
		require(!asks_count(2070000, 500), "throwing stars drop as a whole stack");
		require(!asks_count(2330000, 500), "bullets drop as a whole stack");
		require(!asks_count(1302000, -1), "equipment drops as one");
		require(!asks_count(5000000, 3), "cash items never ask");

		std::cout << "Item drop passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
