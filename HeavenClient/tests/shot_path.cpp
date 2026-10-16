#include "../Gameplay/Combat/ShotPath.h"
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
		using namespace shot_path;

		Point<int16_t> left = origin({ 100, 200 }, 65, true);
		require(left.x() == 35 && left.y() == 172, "a shot to the left starts 65 px left of the user, 28 px up");
		Point<int16_t> right = origin({ 100, 200 }, 30, false);
		require(right.x() == 130 && right.y() == 172, "a shot to the right starts in front on the right");

		Point<int16_t> hit = hit_point(left, -300, { -200, -100, 100, 200 });
		require(hit.x() == -150 && hit.y() == 172, "a shot hits the middle of a body across its row");

		hit = hit_point(left, -300, { 0, 80, 100, 200 });
		require(hit.x() == 17, "a body reaching behind the origin is hit between it and the origin");

		hit = hit_point(left, -300, { -200, -100, 100, 150 });
		require(hit.y() == 149, "a body above the row is hit at its bottom");

		hit = hit_point(left, -300, { -400, -350, 100, 200 });
		require(hit.x() == -350, "a body past the end of the row is hit at its near edge");

		require(flight_time({ 0, 0 }, { 300, 40 }) == 453, "a bullet flies 1.5 ms per pixel of the straight path");
		require(hit_delay(left, { -150, 172 }) == 277, "a hit lands 1.5 ms per pixel across");
		require(hit_delay(left, { -150, 100 }) == 277, "only the horizontal distance times the hit");

		require(spread(0, 1) == 0, "a single bullet flies at its target");
		require(spread(0, 2) == -7 && spread(1, 2) == 7, "two bullets fly 7 px either side");
		require(spread(0, 3) == -14 && spread(2, 3) == 14, "three bullets fly 14 px apart");

		require(bullet_delay(2070000, 0, 1) == 120, "throwing stars leave 120 ms apart");
		require(bullet_delay(5021000, 0, 2) == 240, "cash shop stars do too");
		require(bullet_delay(2060000, 0, 1) == 0, "arrows leave together");
		require(bullet_delay(2060000, 3111006, 3) == 180, "Strafe's arrows leave 60 ms apart");
		require(bullet_delay(2330000, 5001003, 1) == 240, "Double Shot's bullets leave 240 ms apart");

		require(range_skill(Weapon::Type::BOW, 320) == 3000002, "a hunter's bow reaches further with Eye of Amazon");
		require(range_skill(Weapon::Type::CROSSBOW, 1310) == 13000001, "a wind archer's uses the Cygnus one");
		require(range_skill(Weapon::Type::CLAW, 410) == 4000001, "a claw reaches further with Keen Eyes");
		require(range_skill(Weapon::Type::GUN, 510) == 0, "a gun has no range skill");

		require(range(0, 0, Weapon::Type::BOW, 120) == 420, "a bow reaches 300 px plus Eye of Amazon's range");
		require(range(0, 0, Weapon::Type::CLAW, 0) == 200, "a claw reaches 200 px without Keen Eyes");
		require(range(0, 0, Weapon::Type::GUN, 100) == 200, "a gun always reaches 200 px");
		require(range(5001003, 350, Weapon::Type::GUN, 0) == 350, "a skill's own range wins");
		require(range(0, 0, Weapon::Type::SWORD_1H, 0) == 0, "a sword shoots nowhere");

		require(aimed_spell(2001004), "Energy Bolt is aimed at the first mob in its path");
		require(aimed_spell(2001005), "so is Magic Claw");
		require(!aimed_spell(2201005), "Thunder Bolt hits all the mobs in its area");
		require(aimed_spell(2221006), "Chain Lightning's first mob is the first in its path");

		std::vector<Rectangle<int16_t>> steps = search_steps(100, { 165, 172 }, 300);
		require(steps.size() == 7, "a shot searches 20 px steps up to its range");
		require(steps[0].left() == 165 && steps[0].right() == 185, "the first step starts at the shot's origin");
		require(steps[0].top() == 156 && steps[0].bottom() == 188, "it reaches 1 px up and down per 4 px from the user");
		require(steps[6].left() == 285 && steps[6].right() == 300, "the last step stops at the range");
		require(steps[6].top() == 126 && steps[6].bottom() == 218, "the steps widen further away");

		steps = search_steps(100, { 100, 172 }, -100);
		require(steps.size() == 10, "a skill's shot to the left starts at the user");
		require(steps[0].left() == 80 && steps[0].right() == 100, "the first step is left of the user");
		require(steps[0].top() == 172 && steps[0].bottom() == 172, "and empty");
		require(steps[1].top() == 167 && steps[1].bottom() == 177, "the next is 5 px each way");

		require(!in_step(steps[0], { 70, 110, 100, 200 }), "nothing is in an empty step");
		require(in_step(steps[1], { 50, 70, 176, 200 }), "a body overlapping a step is in it");
		require(!in_step(steps[1], { 50, 70, 177, 200 }), "a body only touching a step's bottom is not");
		require(!in_step(steps[1], { 40, 60, 100, 200 }), "a body only touching its side is not");

		Line floor = { 0, 100, 200, 100 };
		Point<int16_t> to = { 150, 150 };
		require(blocked({ 50, 50 }, to, { { floor, 1 } }, 2), "a shot down through a floor stops");
		require(to.x() == 100 && to.y() == 100, "where it meets the floor");
		to = { 50, 50 };
		require(!blocked({ 150, 150 }, to, { { floor, 1 } }, 2), "a shot up through a floor goes on");
		require(to.x() == 50 && to.y() == 50, "to where it was going");

		Line wall = { 100, 0, 100, 100 };
		to = { 50, 50 };
		require(blocked({ 150, 50 }, to, { { wall, 3 } }, 3), "a shot into a wall of the user's group stops");
		require(to.x() == 100 && to.y() == 50, "at the wall");
		to = { 50, 50 };
		require(!blocked({ 150, 50 }, to, { { wall, 3 } }, 5), "a wall of another group lets it through");
		to = { 50, 50 };
		require(blocked({ 150, 50 }, to, { { wall, 0 } }, 5), "a wall of no group does not");
		to = { 150, 50 };
		require(!blocked({ 50, 50 }, to, { { wall, 3 } }, 3), "a shot out of a wall's back goes on");

		to = { 50, 150 };
		require(blocked({ 50, 50 }, to, { { floor, 1 }, { { 0, 80, 200, 80 }, 1 } }, 1), "a shot through two floors");
		require(to.y() == 80, "stops at the nearer");

		Line low = { 0, 100, 100, 100 };
		Line rest = { 100, 100, 200, 100 };
		to = { 100, 150 };
		require(blocked({ 100, 50 }, to, { { low, 1, std::nullopt, rest } }, 1), "a shot down where two floors meet stops");
		require(to.y() == 100, "where they meet");
		to = { 100, 150 };
		require(!blocked({ 100, 50 }, to, { { low, 1 } }, 1), "a shot down past a floor's loose end goes on");

		Line ledge = { 100, 100, 100, 200 };
		to = { 50, 150 };
		require(blocked({ 150, 50 }, to, { { low, 1, std::nullopt, ledge } }, 1), "a shot into a ledge's tip stops");
		to = { 150, 150 };
		require(!blocked({ 50, 50 }, to, { { low, 1, std::nullopt, ledge } }, 1), "one grazing it goes on");

		std::cout << "Shot path passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
