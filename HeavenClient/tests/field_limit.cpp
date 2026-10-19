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

		require(may_summon(0, {}, 2100000), "a summoning sack opens on an open map");
		require(!may_summon(0x4, {}, 2100000), "bit 0x4 keeps it shut");
		require(may_summon(0x526fc, allowed, 2109004), "unless the map lists it");
		require(!may_summon(0x526fc, allowed, 2100000), "an unlisted one stays shut");
		require(sack_asks(2100000) && sack_asks(2101000), "a sack asks first");
		require(!sack_asks(2109004) && !sack_asks(2100067), "an event's sack and 2100067 don't");

		require(may_drop(0x400), "consumable limits leave drops alone");
		require(!may_drop(0x400000), "bit 0x400000 keeps items on the player");

		require(may_jump(0, false) && may_jump(0, true), "an open map allows both jumps");
		require(!may_jump(0x1, false) && may_jump(0x1, true), "bit 0x1 stops only the plain jump");
		require(may_jump(0x20000, false) && !may_jump(0x20000, true), "bit 0x20000 stops only the down-jump");

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

		require(may_use_wheel(0, henesys) && may_use_wheel(2, 104000100) && may_use_wheel(6, 211041400),
				"a Wheel of Destiny works in ordinary fields");
		require(!may_use_wheel(1, henesys) && !may_use_wheel(4, henesys) && !may_use_wheel(15, henesys),
				"but not in some field types");
		require(!may_use_wheel(0, 910000000) && !may_use_wheel(0, 200090010) && !may_use_wheel(0, 390000000),
				"nor in event maps, on the ships or in 390xxxxxx");

		require(may_touch(0, false, 100100) && !may_touch(0, true, 100100), "Dark Sight keeps a snail's touch off");
		require(may_touch(0x2, true, 100100), "but not where bit 0x2 is set");
		require(may_touch(0, true, MASTER_GUARDIAN), "and never the Master Guardian's");

		require(may_attack(0, 1121006) && !may_attack(0x2, 1121006), "bit 0x2 stops Rush");
		require(!may_attack(0x2, 4211002) && !may_attack(0x2, 5201006), "Assaulter and Recoil Shot too");
		require(may_attack(0x2, 0) && may_attack(0x2, 3221007), "but not a plain attack or Snipe");

		require(may_teleport_rock(0, henesys), "a teleport rock works in Henesys");
		require(!may_teleport_rock(0x40, henesys) && !may_teleport_rock(0x2, henesys), "but not with bit 0x40 or 0x2");
		require(!may_teleport_rock(0, 109010000) && !may_teleport_rock(0, 209000000), "nor in the event maps");

		require(may_change_leader(0x2) && !may_change_leader(0x800), "bit 0x800 keeps the party's leader");

		require(shows_minimap(false, true, false, false, true), "a map's miniMap shows");
		require(!shows_minimap(false, false, false, false, true) && !shows_minimap(true, false, false, false, true),
				"a map without one folds, even for a GM");
		require(!shows_minimap(false, true, true, false, true) && shows_minimap(true, true, true, false, true),
				"hideMinimap folds it for all but GMs");
		require(shows_minimap(false, true, false, true, true) && !shows_minimap(false, true, false, true, false),
				"miniMapOnOff folds it once the server switches minimaps off");
		require(shows_minimap(true, true, false, true, false) && shows_minimap(false, true, false, false, false),
				"but not for GMs, nor in other maps");

		std::cout << "Field limit passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
