#include "../Gameplay/MapleMap/MistLayout.h"
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

		// With every roll 0, columns are 35 px apart, and a 50 px tile piles
		// 40 px below the last. A column's top and bottom are inset by half the
		// area's height times its distance from the centre over the width.
		std::vector<int32_t> asked;
		auto lowest = [&](int32_t n) { asked.push_back(n); return 0; };
		std::vector<MistCloud> clouds = layout_mist(0, 0, 100, 100, { 50 }, lowest);
		std::vector<MistCloud> want = {
			{ 0, 0, 25, 0, false }, { 0, 0, 65, 0, false },
			{ 0, 35, 7, 0, false }, { 0, 35, 47, 0, false }, { 0, 35, 87, 0, false },
			{ 0, 70, 10, 0, false }, { 0, 70, 50, 0, false }
		};
		require(clouds == want, "a diamond of columns");
		require(std::vector<int32_t>(asked.begin(), asked.begin() + 7) == std::vector<int32_t>{ 1, 10, 2, 1, 10, 2, 20 },
				"tile, order and flip per cloud, then the column's step");

		// With every roll its highest, the tall tile, the top order, flipped,
		// and columns 54 px apart.
		auto highest = [](int32_t n) { return n - 1; };
		clouds = layout_mist(0, 0, 100, 100, { 50, 100 }, highest);
		want = { { 1, 0, 25, 9, true }, { 1, 54, 2, 9, true }, { 1, 54, 82, 9, true } };
		require(clouds == want, "the highest rolls");

		// Coordinates truncate toward zero, as the client's ftol does.
		clouds = layout_mist(-200, -150, 200, 150, { 95 }, lowest);
		require(clouds.front().x == -200 && clouds.front().y == -75, "the first cloud at the left edge's top");
		require(clouds[1].y == 1, "-75 + 76 is the next");
		require(layout_mist(0, 0, 0, 100, { 50 }, lowest).empty(), "an empty area has no clouds");

		// A Commander Skeleton's Poison Mist (131 level 5) hurts its x, as
		// poison, once started; the code is the skill's byte, then the level.
		mist::Area poison{ mist::MOB, 2000001, 131, 5, -135, -80, 135, 40, 4, 200, true };
		std::vector<mist::Area> areas{ poison };
		mist::Hurt hurt{};
		require(mist::hurt_at(areas, 0, 0, hurt), "standing in it hurts");
		require(hurt.damage == 200 && hurt.element == 4 && hurt.code == (131 << 8 | 5), "its x, element and code");
		require(mist::hurt_at(areas, -135, -80, hurt), "its top left is in");
		require(!mist::hurt_at(areas, 135, 0, hurt) && !mist::hurt_at(areas, 0, 40, hurt),
				"its right and bottom edges are out (PtInRect)");
		areas[0].started = false;
		require(!mist::hurt_at(areas, 0, 0, hurt), "nor before its delay is over");
		areas[0] = poison;
		areas[0].type = 1;
		require(!mist::hurt_at(areas, 0, 0, hurt), "only a mob's mist hurts");
		areas[0] = poison;
		areas[0].damage = 0;
		require(!mist::hurt_at(areas, 0, 0, hurt), "one that does nothing doesn't");
		require(mist::hurts(130) && mist::hurts(131) && !mist::hurts(2111003), "the hurting mob skills");
		require(mist::resisted_as(130) == 2 && mist::resisted_as(125) == 4 && mist::resisted_as(131) == 0,
				"a fire mist is resisted as fire, Poison Mist not at all");
		require(mist::resisted(200, 40) == 120 && mist::resisted(7, 50) == 3 && mist::resisted(200, 0) == 200,
				"the rest of the damage, rounded down");

		// A smoke screen shelters its owner and their party, not others.
		mist::Area smoke{ mist::SMOKE, 7, 4221006, 20, -100, -100, 100, 100, 0, 0, true };
		std::vector<mist::Area> smokes{ smoke };
		auto ally = [](int32_t owner) { return owner == 7; };
		auto stranger = [](int32_t) { return false; };
		require(mist::shelters(smokes, 0, 0, ally), "an ally's smoke shelters");
		require(!mist::shelters(smokes, 0, 0, stranger), "a stranger's doesn't");
		require(!mist::shelters(smokes, 200, 0, ally), "nor outside it");
		require(!mist::shelters(areas, 0, 0, ally), "a poison mist doesn't");

		// A cash mist waits for the user's own from the item, or while
		// they stand in any.
		std::vector<mist::Area> gas{ { mist::ITEM, 7, 5281000, 1, -110, -82, 110, 83, 0, 0, true } };
		require(mist::item_busy(gas, 7, 5281000, 500, 0), "one's own gas lasts");
		require(!mist::item_busy(gas, 7, 5281001, 500, 0), "another item's free");
		require(mist::item_busy(gas, 8, 5281001, 0, 0), "standing in anyone's waits");
		require(!mist::item_busy(gas, 8, 5281000, 110, 0), "the right edge is out");
		require(!mist::item_busy(smokes, 7, 4221006, 0, 0), "a smoke screen is no cash mist");
		gas[0].started = false;
		require(!mist::item_busy(gas, 7, 5281000, 0, 0), "nor is one not started yet");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
