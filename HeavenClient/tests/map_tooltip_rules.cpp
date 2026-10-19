#include "../Gameplay/MapleMap/MapTooltipRules.h"
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
		using namespace ms::map_tooltips;

		ZoneName zero = parse_zone_name("0");
		ZoneName ten = parse_zone_name("10char");
		ZoneName bare = parse_zone_name("char");
		require(!zero.character && zero.key == 0, "a numbered zone");
		require(ten.character && ten.key == 10, "zone 10's character zone");
		require(bare.character && bare.key == 0, "atoi of nothing is 0");

		// Maple Island's 10000 zone 0.
		Zone portal = { 656, 88, 750, 225 };
		require(contains(portal, 656, 88) && contains(portal, 749, 224), "left and top edges are in");
		require(!contains(portal, 750, 100) && !contains(portal, 700, 225) && !contains(portal, 655, 100),
				"right and bottom edges are out");

		// Henesys' (100000000) zones 3 and 4 and their character zones.
		std::map<int32_t, Zone> zones = { { 3, { 4757, -161, 4799, -143 } }, { 4, { 4738, -186, 4779, -169 } },
										  { 5, { 4700, -200, 4800, -100 } } };
		std::map<int32_t, Zone> chars = { { 3, { 4707, -235, 4843, -90 } }, { 4, { 4706, -233, 4841, -91 } } };
		require(zone_at(zones, 4760, -150) == 3, "the first zone the cursor is over");
		require(zone_at(zones, 4710, -150) == 5, "another");
		require(zone_at(zones, 0, 0) == -1, "none");
		require(char_zones_at(chars, 4750, -150) == std::vector<int32_t>({ 3, 4 }), "both character zones");
		require(char_zones_at(chars, 4706, -150) == std::vector<int32_t>({ 4 }), "only 4char's left edge");
		require(char_zones_at(chars, 4843, -150).empty(), "past both right edges");
		require(anchor(zones[3]) == std::make_pair(4799, -143) && anchor(zones[4]) == std::make_pair(4779, -169),
				"a balloon hangs from its zone's bottom right");

		Box title = compact_box(90, 0);
		require(title.width == 104 && title.height == 22, "a title alone");
		require(title.title_x == 7 && title.title_y == 4, "the title centred");
		Box both = compact_box(90, 30);
		require(both.width == 104 && both.height == 56, "the description under it");
		require(both.desc_x == 10 && both.desc_y == 26 && both.wrap == 89, "wrapped inside the title's width");
		require(compact_box(0, 30).width == 0, "nothing without a title");

		Box line = line_box(50);
		require(line.width == 58 && line.height == 18 && line.title_x == 4 && line.title_y == 1, "a portal's tip");

		require(clamp_screen(790, 220, 104, 22, 800, 600) == std::make_pair<int16_t, int16_t>(695, 220),
				"kept off the right edge");
		require(clamp_screen(-5, 590, 104, 22, 800, 600) == std::make_pair<int16_t, int16_t>(0, 577),
				"and the left and bottom ones");
		require(clamp_screen(100, 100, 104, 22, 800, 600) == std::make_pair<int16_t, int16_t>(100, 100),
				"where it was put");

		Zone spot = minimap_portal_spot(50, 40);
		require(spot.left == 48 && spot.top == 35 && spot.right == 53 && spot.bottom == 42, "5 x 7 at the marker");
		require(contains(spot, 48, 35) && contains(spot, 52, 41), "in the spot");
		require(!contains(spot, 53, 40) && !contains(spot, 50, 42), "out of it");

		require(minimap_shows(2) && minimap_shows(7) && !minimap_shows(8), "visible portals");
		require(minimap_tip(7, 100000000, false) && minimap_tip(2, 999999999, false), "a tip");
		require(!minimap_tip(8, 100000000, false) && !minimap_tip(7, -1, false) && !minimap_tip(7, 100000000, true),
				"none for a hidden portal, one leading nowhere or one with its tip hidden");

		std::cout << "map tooltip rules ok\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}
}
