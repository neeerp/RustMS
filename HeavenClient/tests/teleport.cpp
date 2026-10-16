#include "../Gameplay/Combat/Teleport.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	using ms::Point;
	using ms::Rectangle;
	using ms::teleport::Floor;
	using ms::teleport::Spot;

	Floor floor(uint16_t id, int32_t x1, int32_t y1, int32_t x2, int32_t y2)
	{
		return { id, { x1, y1, x2, y2 } };
	}

	bool lands(std::optional<Spot> spot, int16_t x, int16_t y, uint16_t foothold)
	{
		return spot && spot->position.x() == x && spot->position.y() == y && spot->foothold == foothold;
	}

	bool clear(Point<int16_t>, Point<int16_t>)
	{
		return true;
	}

	bool blocked(Point<int16_t>, Point<int16_t>)
	{
		return false;
	}

	// A 40 px wide, 60 px tall mob standing at (x, y).
	ms::teleport::Mob mob(int16_t x, int16_t y, uint32_t entry, bool blocks)
	{
		return { { static_cast<int16_t>(x - 20), static_cast<int16_t>(x + 20), static_cast<int16_t>(y - 60), y }, entry, blocks };
	}
}

int main()
{
	try
	{
		using namespace ms;
		using namespace teleport;

		require(teleports(2101002) && teleports(2201002) && teleports(2301001) && teleports(9001002),
				"the mage and GM Teleports teleport");
		require(!teleports(2101001) && !teleports(4111006), "other skills don't");

		// Floors: ground, a platform above it, a ledge past the ground's end,
		// a wall and a ceiling.
		std::vector<Floor> floors = {
			floor(1, -500, 0, 500, 0),
			floor(2, -100, -120, 100, -120),
			floor(3, 600, -50, 800, -50),
			floor(4, 550, -200, 550, 100),
			floor(5, 900, -300, 700, -300)
		};

		// The height of a floor is cut toward zero.
		require(height({ 0, 0, 3, 10 }, 1) == 3, "a sloped floor's height is cut toward zero");
		require(!height({ 0, 0, 0, 10 }, 0), "a wall is no floor");
		require(!height({ 10, 0, 0, 0 }, 5), "nor is a ceiling");
		require(!height({ 0, 0, 10, 0 }, 11), "nor is a floor that doesn't reach");

		// The floor below starts at y and stops short of the bottom.
		require(floor_below(floors, 0, -120, 0)->id == 2, "a floor at y is below");
		require(!floor_below(floors, 0, -119, 0), "the bottom is not");
		require(!floor_below(floors, 0, 10, 0), "nor is anything when the bottom is above");
		require(floor_above(floors, 0, 0, -120)->id == 1, "a floor at y is above");
		require(!floor_above(floors, 0, -1, -120), "the top is not");

		// Across: the floor at the same height.
		require(lands(destination({ 0, 0 }, 1, 1, 0, 150, floors, clear), 150, -1, 1),
				"across, it lands 1 px above the floor the way pressed");
		require(lands(destination({ 0, 0 }, 1, -1, 1, 150, floors, clear), -150, -1, 1),
				"across, up or down is ignored");

		// Up to 80 px up or down, past the end of a floor.
		require(lands(destination({ 450, 0 }, 1, 1, 0, 200, floors, clear), 650, -51, 3),
				"across, it lands on a ledge up to 80 px up");
		require(!destination({ 450, 0 }, 1, 1, 0, 200, { floor(1, -500, 0, 500, 0), floor(3, 600, -80, 800, -80) }, clear),
				"but not 80 px up");
		require(lands(destination({ 450, 0 }, 1, 1, 0, 200, { floor(1, -500, 0, 500, 0), floor(3, 600, 79, 800, 79) }, clear), 650, 78, 3),
				"and up to 80 px down");
		require(!destination({ 450, 0 }, 1, 1, 0, 200, { floor(1, -500, 0, 500, 0), floor(3, 600, 80, 800, 80) }, clear),
				"but not 80 px down");

		// The nearer of the floors up and down, the one down on a tie.
		std::vector<Floor> both = { floor(1, -500, 0, 500, 0), floor(6, 600, 60, 800, 60), floor(7, 600, -30, 800, -30) };
		require(lands(destination({ 450, 0 }, 1, 1, 0, 200, both, clear), 650, -31, 7), "the nearer floor up wins");
		both[1] = floor(6, 600, 30, 800, 30);
		require(lands(destination({ 450, 0 }, 1, 1, 0, 200, both, clear), 650, 29, 6), "the floor down wins a tie");

		// No way pressed, or nothing there.
		require(!destination({ 0, 0 }, 1, 0, 0, 150, floors, clear), "with no way pressed it goes nowhere");
		require(!destination({ 0, 0 }, 1, 1, 0, 1000, floors, clear), "with no floor there it goes nowhere");

		// Up: onto a platform between, if there is something in the way.
		Point<int16_t> from, to;
		auto watch = [&](Point<int16_t> f, Point<int16_t> t)
		{
			from = f;
			to = t;
			return false;
		};
		require(lands(destination({ 0, 0 }, 1, 0, -1, 150, floors, watch), 0, -121, 2), "up, it lands on the platform above");
		require(from.x() == 0 && from.y() == -121 && to.x() == 0 && to.y() == -1, "looking from there down to the user");
		require(lands(destination({ 0, 0 }, 1, 0, -1, 150, floors, clear), 0, -1, 1),
				"with nothing in the way, it stays where it is");
		require(lands(destination({ 0, 0 }, 1, 0, -1, 100, floors, blocked), 0, -1, 1),
				"too short to reach the platform, it stays where it is");
		require(lands(destination({ 0, 0 }, 9, 0, -1, 150, { floor(1, -500, 0, 500, 0) }, blocked), 0, -1, 9),
				"staying, it stays on its own floor");
		require(!destination({ 0, 0 }, 1, 0, -1, 150, { floor(2, 100, -120, 300, -120) }, clear),
				"with no floor below where it would go, it goes nowhere");

		// Down: onto the floor at or above where it would go.
		require(lands(destination({ 0, -120 }, 2, 0, 1, 150, floors, clear), 0, -1, 1), "down, it lands on the floor below");
		require(lands(destination({ 0, -120 }, 2, 0, 1, 120, floors, clear), 0, -1, 1), "even right at its end");
		require(lands(destination({ 0, -120 }, 2, 0, 1, 119, floors, clear), 0, -1, 1),
				"or past it, looking down from its end");
		require(!destination({ 0, -120 }, 2, 0, 1, 50, floors, clear), "but not range past it");

		// Unless that's its own floor, and then up to range below that.
		std::vector<Floor> deep = { floor(1, -500, 0, 500, 0), floor(8, -500, 200, 500, 200) };
		require(lands(destination({ 0, 0 }, 1, 0, 1, 150, deep, clear), 0, 199, 8), "from its own floor, it looks further down");
		require(!destination({ 0, 0 }, 1, 0, 1, 100, deep, clear), "but only range further");
		require(!destination({ 0, 0 }, 1, 0, 1, 150, { floor(1, -500, 0, 500, 0) }, clear),
				"and with nothing there it goes nowhere");

		// Where a mob can stop it.
		std::optional<Rectangle<int16_t>> area = mob_area({ 0, 0 }, 1, 0, 150);
		require(area && area->left() == 0 && area->right() == 150 && area->top() == -65 && area->bottom() == 0,
				"across, the way to the end, 65 px up");
		area = mob_area({ 0, 0 }, -1, 0, 150);
		require(area && area->left() == -150 && area->right() == 0, "either way");
		area = mob_area({ 0, 0 }, 0, -1, 150);
		require(area && area->left() == -20 && area->right() == 20 && area->top() == -215 && area->bottom() == 0,
				"up, 20 px either side, from 65 px above the end");
		area = mob_area({ 0, 0 }, 0, 1, 150);
		require(area && area->top() == -65 && area->bottom() == 150, "down, from 65 px above the user");
		require(!mob_area({ 0, 0 }, 0, 0, 150), "with no way pressed, nowhere");

		Rectangle<int16_t> way = *mob_area({ 0, 0 }, 1, 0, 150);
		require(mob_in_way(way, { mob(100, 0, 0, true) }), "a mob that can't be passed stops it");
		require(!mob_in_way(way, { mob(100, 0, 0, false) }), "others don't");
		require(!mob_in_way(way, { mob(200, 0, 0, true) }), "nor one out of the way");

		std::vector<ms::teleport::Mob> crowd;

		for (uint32_t i = 0; i < 15; i++)
			crowd.push_back(mob(100, 0, 1 + i, false));

		crowd.push_back(mob(50, 0, 50, true));
		require(!mob_in_way(way, crowd), "one behind the first 15 in the way is missed");
		crowd.back().entry = 0;
		require(mob_in_way(way, crowd), "one among them isn't");

		std::cout << "Teleport passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
