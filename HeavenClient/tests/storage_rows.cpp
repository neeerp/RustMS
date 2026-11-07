#include "../IO/StorageRows.h"
#include <iostream>
#include <stdexcept>

using namespace ms::storage_rows;

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		std::vector<Entry> list = { { 2000000, 5, 1 }, { 2000001, 7, 2 }, { 2000002, 9, 4 } };

		// A stack grown, or an item in a slot of its own, is the row that differs.
		auto grown = list;
		grown[1].count = 8;
		require(changed_row(list, grown) == 1, "the stack that grew");

		auto moved = list;
		moved[2].slot = 3;
		require(changed_row(list, moved) == 2, "a slot counts");

		auto added = list;
		added.insert(added.begin() + 1, { 2000005, 1, 2 });
		require(changed_row(list, added) == 1, "an item among the others");

		auto appended = list;
		appended.push_back({ 2000005, 1, 5 });
		require(changed_row(list, appended) == 3, "an item at the end");
		require(changed_row({}, { { 2000005, 1, 1 } }) == 0, "the first item");

		// Nothing changed, and a list that only got shorter.
		require(changed_row(list, list) == 0, "unchanged");
		require(changed_row(list, { list[0], list[1] }) == 1, "the last row left");

		// The top moves only for a row above it, or past four below it.
		require(scrolled_to(1, 3) == 1, "up to a row above");
		require(scrolled_to(3, 3) == 3, "the top row");
		require(scrolled_to(6, 3) == 3, "the fourth row");
		require(scrolled_to(7, 3) == 3, "one past the last shown stays out of view");
		require(scrolled_to(8, 3) == 4, "down to a row far below");

		// Equip to cash, with the tab counted from 0.
		require(switches_tab(1, 2), "the item's tab sets itself again");
		require(switches_tab(4, 2), "a far tab switches");
		require(switches_tab(0, 2), "a tab to the left switches");
		require(!switches_tab(2, 2), "the tab right of the item's stays");
		require(switches_tab(4, 5), "cash has no tab right of it");

		// Four slots still show five rows; beyond that, every slot.
		require(stored_positions(4, 5) == 5, "fewer slots than rows");
		require(stored_positions(5, 5) == 5, "as many as rows");
		require(stored_positions(6, 5) == 6, "one to scroll");
		require(stored_positions(48, 5) == 48, "every slot");

		std::cout << "storage rows ok\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
