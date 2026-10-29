#include "../Character/CharInfoLists.h"
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
		using namespace ms::char_info;
		using Items = std::vector<int32_t>;

		// A cash hat over a hat, a weapon under a cash one, a pet's equipment, and the mount's parts.
		std::map<int8_t, int32_t> equips = {
			{ 1, 1002140 }, { 5, 1040002 }, { 11, 1302000 }, { 14, 1802000 }, { 18, 1902000 }, { 19, 1912000 },
			{ 23, 1812000 }, { 49, 1142000 }
		};
		std::map<int8_t, int32_t> masked = { { 1, 1002000 }, { -111, 1702000 } };

		require(worn_items(equips, masked) == Items{ 1002000, 1002140, 1040002, 1702000, 1302000, 1142000 },
				"each part's covered item, then the one shown, the cash weapon first, less the pet's and the mount's");
		require(worn_items({}, { { -111, 1302000 } }).empty(), "only a cash weapon is listed for the weapon");

		auto find = [](int32_t sn) -> std::optional<Wish>
		{
			if (sn == 1)
				return Wish{ 1, 5000000, 1 };
			if (sn == 2)
				return Wish{ 2, 0, 1 };
			if (sn == 3)
				return Wish{ 3, 2000000, 10 };

			return std::nullopt;
		};
		std::vector<Wish> rows = wishes({ 1, 9, 2, 3 }, find);
		require(rows.size() == 2 && rows[0].sn == 3 && rows[1].sn == 1, "last first, less the unknown and no item");
		require(wish_name("Red Potion", 10) == "Red Potion (10)" && wish_name("Brown Kitty", 1) == "Brown Kitty",
				"a count above 1 follows the name");

		require(last_top(5, 3) == 2 && last_top(3, 3) == 0 && last_top(0, 3) == 0, "the list scrolls to its end");

		auto medal = [](int16_t quest) { return quest == 29002 ? 0 : 1140000 + quest - 29000; };
		std::vector<int16_t> quests = { 29000, 29001, 29002, 29003, 29004 };
		require(medal_rows(quests, 0, medal) == Items{ 1140000, 1140001, 1140003 }, "a quest with no medal takes no row");
		require(medal_rows(quests, 1, medal) == Items{ 1140001, 1140003, 1140004 }, "from the scrolled place");
		require(medal_rows(quests, 9, medal).empty(), "past the end");

		require(level_icon(1, 8) == 0 && level_icon(8, 8) == 7, "level N shows icon N - 1");
		require(!level_icon(9, 8) && !level_icon(0, 8), "no icon past the last");

		std::cout << "Char info lists passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
