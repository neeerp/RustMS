#include "../Gameplay/Combat/MesoExplosion.h"
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
		using namespace ms::meso_explosion;

		// At level 30, facing left: 300 px behind to the feet, 120 up and 50
		// down; facing right, the other way.
		Rectangle<int16_t> range(-300, 0, -120, 50);
		Rectangle<int16_t> left = area(range, { 1000, 200 }, true);
		Rectangle<int16_t> right = area(range, { 1000, 200 }, false);
		require(left.left() == 700 && left.right() == 1000 && left.top() == 80 && left.bottom() == 250, "faces left");
		require(right.left() == 1000 && right.right() == 1300, "mirrored when facing right");
		require(in_area(left, { 700, 80 }) && in_area(left, { 999, 249 }), "its left and top edges are in");
		require(!in_area(left, { 1000, 200 }) && !in_area(left, { 800, 250 }), "its right and bottom edges are out");

		require(may_set_off(0, 7, 7, 0) && !may_set_off(0, 8, 7, 0), "a character's own mesos");
		require(may_set_off(1, 3, 7, 3) && !may_set_off(1, 4, 7, 3), "a party's");
		require(may_set_off(2, 8, 7, 0) && may_set_off(3, 8, 7, 0), "anyone's and explosive ones");

		std::vector<Drop> found;

		for (int32_t i = 0; i < 25; i++)
			found.push_back({ 100 + i, 1000 - i * 10 + (i == 3 ? 10 : 0), { static_cast<int16_t>(i * 10), 0 } });

		std::vector<Drop> chosen = choose(found, 12);
		require(chosen.size() == 12, "its attack count of them");
		require(chosen.front().oid == 111 && chosen.back().oid == 100, "cheapest first, of the first to enter");
		require(chosen[10].oid == 101 && chosen[11].oid == 100, "the rest after");
		require(choose(found, 30).size() == 20 && choose(found, 0).size() == 1, "between 1 and 20 of them");

		std::vector<Drop> same = { { 1, 50, {} }, { 2, 40, {} }, { 3, 50, {} } };
		std::vector<Drop> sorted = choose(same, 10);
		require(sorted[0].oid == 2 && sorted[1].oid == 1 && sorted[2].oid == 3, "equal amounts keep their order");

		// Drops 100 px apart, and mobs among them.
		std::vector<Drop> drops = { { 1, 10, { 0, 0 } }, { 2, 20, { 100, 0 } }, { 3, 30, { 400, 0 } } };
		std::vector<Mob> mobs = {
			{ 10, Rectangle<int16_t>(40, 60, -60, 0) },
			{ 11, Rectangle<int16_t>(200, 260, -60, 0) },
			{ 12, Rectangle<int16_t>(140, 160, -60, 0) },
			{ 13, Rectangle<int16_t>(380, 420, -60, 0) }
		};
		Targets targets = aim(drops, mobs, 6);
		require(targets.mobs.size() == 3, "only mobs a drop hits");
		require(targets.mobs[0] == std::make_pair(10, 0b011u), "the first hit by the first two");
		require(targets.mobs[1] == std::make_pair(12, 0b010u), "by the second alone");
		require(targets.mobs[2] == std::make_pair(13, 0b100u), "by the third alone");
		require(targets.drops[0] == 0b001 && targets.drops[1] == 0b011 && targets.drops[2] == 0b100,
				"each drop names the mobs it hit");
		require(aim(drops, mobs, 2).mobs.size() == 2, "no more mobs than its mob count");
		require(aim({ { 1, 10, { 0, 0 } } }, { { 10, Rectangle<int16_t>(50, 60, -60, 0) } }, 6).mobs.empty(),
				"a blast's right edge is out");

		std::vector<Drop> many;

		for (int32_t i = 0; i < 20; i++)
			many.push_back({ i, i + 1, { 0, 0 } });

		Targets crowd = aim(many, { { 10, Rectangle<int16_t>(-10, 10, -10, 10) } }, 6);
		require(crowd.mobs[0].second == 0x7FFF, "no more than 15 drops on a mob");
		require(hurting(many, 0xFFFFF).size() == 15 && hurting(many, 0b101) == std::vector<int32_t>({ 1, 3 }),
				"and only the first 15 hurt it");

		require(line(1000, 1000, 0.999999) == 7999, "a 1000-meso line at level 30 is under 8000");
		require(line(1000, 50000, 0.5) == 22624, "half of 45249 for 50000 mesos");
		require(line(500, 1, 1.0) == 135, "a single meso still hurts");

		require(fold({ 1, 2, 3 }) == std::vector<int32_t>({ 1, 2, 3 }), "a few lines show as they are");
		require(fold({ 1, 2, 3, 4, 5, 6, 7 }) == std::vector<int32_t>({ 7, 9, 3, 4, 5 }), "more add up by fives");
		require(fold({}).empty(), "no lines, no numbers");

		require(shown(300, 0, true) == 300 && shown(300, 1, true) == 1, "fixed damage shows for a hit");
		require(shown(0, 1, true) == 0 && shown(300, 0, false) == 0, "not for a miss, and nothing if it isn't hurt");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	std::cout << "meso explosion tests passed\n";
	return 0;
}
