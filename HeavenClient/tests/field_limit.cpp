#include "../Gameplay/MapleMap/FieldLimit.h"
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
		using namespace ms::field_limit;

		// The eleven maps that list items all have fieldLimit 0x526fc.
		const std::vector<int32_t> allowed = { 2022539, 2109004 };

		require(may_use(0, {}, 2000000), "a Red Potion works on an open map");
		require(!may_use(0x400, {}, 2000000), "a Red Potion is refused where consumables are limited");
		require(!may_use(0x526fc, allowed, 2000000), "an unlisted potion is refused");
		require(may_use(0x526fc, allowed, 2022539), "a listed item still works");
		require(may_use(0x400, {}, 2030000), "a Return Scroll doesn't go the potion way");
		require(may_use(0x400, {}, 2040000), "a scroll doesn't go the potion way");
		require(!may_use(0x400, {}, 2210000), "a transformation potion is refused");

		require(may_drop(0x400), "consumable limits leave drops alone");
		require(!may_drop(0x400000), "bit 0x400000 keeps items on the player");

		require(is_use_item(2000000) && is_use_item(2450000), "potions and the like go as USE_ITEM");
		require(!is_use_item(2030000) && !is_use_item(2280000) && !is_use_item(2100000),
				"return scrolls, skill books and summon sacks don't");

		// Part of Map/AreaCode.img: Victoria is 1, Ossyria (Orbis, El Nath) 3, Ludibrium 4.
		const std::map<int32_t, int32_t> codes = { { 10, 1 }, { 11, 2 }, { 20, 3 }, { 21, 3 }, { 22, 4 } };
		const int32_t henesys = 100000000, lith = 104000000, orbis = 200000000, el_nath = 211000000;

		require(same_area(henesys, lith, codes), "Henesys and Lith Harbor share Victoria");
		require(same_area(orbis, el_nath, codes), "Orbis and El Nath share Ossyria");
		require(!same_area(henesys, orbis, codes), "Victoria isn't Ossyria");
		require(!same_area(orbis, 220000000, codes), "Ludibrium is its own area");
		require(!same_area(109010000, henesys, codes), "event maps are in no area");
		require(!same_area(200090000, orbis, codes), "the ships are in no area");
		require(!same_area(henesys, 300000000, codes), "an unlisted code is in no area");

		require(return_scroll_refusal(0, lith, henesys, false, codes) == ScrollRefusal::NONE,
				"a Henesys scroll works in Victoria");
		require(return_scroll_refusal(0, orbis, henesys, false, codes) == ScrollRefusal::THERE,
				"but not from Orbis");
		require(return_scroll_refusal(0, orbis, henesys, true, codes) == ScrollRefusal::NONE,
				"unless it ignores continents");
		require(return_scroll_refusal(0, orbis, NEAREST_TOWN, false, codes) == ScrollRefusal::NONE,
				"a nearest-town scroll works anywhere");
		require(return_scroll_refusal(0x100, orbis, NEAREST_TOWN, false, codes) == ScrollRefusal::NONE,
				"bit 0x100 leaves nearest-town scrolls alone");
		require(return_scroll_refusal(0x100, lith, henesys, false, codes) == ScrollRefusal::HERE,
				"but stops scrolls that name a map");
		require(return_scroll_refusal(0x4207c, lith, NEAREST_TOWN, false, codes) == ScrollRefusal::HERE,
				"bit 0x20 stops every return scroll");

		std::cout << "Field limit passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
