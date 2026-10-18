#include "../Gameplay/MapleMap/MobAttackRules.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool same(const ms::mob_attack::Box &a, const ms::mob_attack::Box &b)
{
	return a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom;
}

int main()
{
	try
	{
		using namespace ms::mob_attack;

		require(!intersects({ 0, 0, 10, 10 }, { 10, 0, 20, 10 }), "boxes that share an edge don't meet");
		require(intersects({ 0, 0, 10, 10 }, { 9, 9, 20, 20 }), "one pixel of overlap meets");
		require(!intersects({ 10, 0, 0, 10 }, { -100, -100, 100, 100 }), "an inside-out box meets nothing");
		require(contains({ 0, 0, 10, 10 }, 0, 0) && !contains({ 0, 0, 10, 10 }, 10, 5),
				"a box holds its left edge but not its right one");

		require(same(user_body(100, 200, false, true), { 78, 135, 122, 200 }), "a standing body is 44 by 65");
		require(same(user_body(100, 200, false, false), { 78, 135, 122, 200 }), "and the same either way");
		require(same(user_body(100, 200, true, true), { 54, 169, 100, 200 }), "a prone body lies behind");
		require(same(user_body(100, 200, true, false), { 100, 169, 146, 200 }), "facing either way");

		// Leftward strips: x 0..-20 at height 1, -20..-40 at +-5, -40..-60 at
		// +-10, -60..-80 at +-15.
		require(!cone_reaches(0, -100, 0, { -60, -14, -50, -10 }), "the cone widens a step at a time");
		require(cone_reaches(0, -100, 0, { -61, -14, -50, -10 }), "the next strip is wider");
		require(!cone_reaches(0, -100, 0, { -130, -5, -100, 5 }), "the cone stops at its range");
		require(!cone_reaches(0, 100, 0, { 95, -30, 120, -20 }), "rightward strips widen the same way");
		require(cone_reaches(0, 100, 0, { 95, -30, 120, -19 }), "up to a quarter of the distance");
		require(!cone_reaches(0, 100, 0, { -40, -5, -10, 5 }), "nothing behind the shooter");

		require(choose_facing(true, true, true, true) == std::optional<bool>(true), "facing left tries left first");
		require(choose_facing(true, true, false, true) == std::optional<bool>(false), "then turns right");
		require(!choose_facing(true, false, false, true), "unless it may not turn");
		require(choose_facing(false, false, true, true) == std::optional<bool>(false),
				"facing right tries right first");
		require(choose_facing(false, true, true, false) == std::optional<bool>(true), "then turns left");
		require(!choose_facing(false, true, false, false), "and gives up when neither reaches");

		require(origin_x(100, -30, true) == 70 && origin_x(100, -30, false) == 130, "sp is drawn facing left");

		Box body = user_body(100, 200, false, true);
		require(aim(body, 180, true) == std::make_pair(112, 180), "aim 10 px into the near side");
		require(aim(body, 195, true) == std::make_pair(112, 167), "low shooters aim at the middle");
		require(aim(body, 180, false) == std::make_pair(88, 180), "from the other side too");
		require(aim({ 90, 135, 100, 200 }, 180, true) == std::make_pair(95, 180), "no further in than the middle");

		require(shot_point(body, 300, true) == std::make_pair(112, 200), "shots meet the body at its edge");
		require(shot_reaches(body, 222, 200, 100, true) && !shot_reaches(body, 223, 200, 100, true),
				"shots reach 10 px past their range");

		require(ball_end(200, 180, 112, 180, 300, true) == std::make_pair(-100, 180), "balls fly their full range");
		require(ball_end(200, 180, 190, 100, 300, true) == std::make_pair(-57, 26), "no steeper than 0.6");
		require(ball_end(200, 180, 250, 180, 300, true) == std::make_pair(-100, 180),
				"a target behind is shot straight ahead");
		require(ball_time(300, 0) == 1800 && ball_time(300, 200) == 900, "balls fly at bulletSpeed, 100 by default");

		// 8220006 attack1: lt (-40, -200), rb (40, 0), start -7, areaCount 15.
		Area area{ { -40, -200, 40, 0 }, -7, 15 };
		require(area.width() == 80, "a cell is as wide as the range box");
		require(same(area_box(area, 0, 0, true), { -560, -200, 640, 0 }), "the area spans every cell");
		require(area_box(area, 0, 0, false).empty(), "facing right, the area is inside out");
		require(cell_x(area, 0, 0, true) == -520 && cell_x(area, 0, 14, true) == 600, "cells run left to right");
		require(cell_x(area, 1000, 7, false) == 960, "and right to left facing right");
		require(same(cell_box(area, -520, 10, true), { -560, -190, -480, 10 }), "a cell's hit sits on its middle");

		// Skills.
		require(skill_action(1) == 21 && skill_action(3) == 23, "skill1 is action 21");
		require(skill_data(114, 12, 1260) == (114 | 12 << 8 | 1260 << 16), "skill, level, then duration");
		require(cast_skill(skill_data(200, 27, 900)) == std::make_pair<uint8_t, uint8_t>(200, 27),
				"a cast reads back its skill and level");
		require(granted_skill({ 0, 0 }, { 100, 1 }) == std::make_pair<uint8_t, uint8_t>(100, 1), "a grant is kept");
		require(granted_skill({ 100, 1 }, { 0, 0 }) == std::make_pair<uint8_t, uint8_t>(100, 1),
				"until another ack grants something");
		require(granted_skill({ 100, 1 }, { 114, 12 }) == std::make_pair<uint8_t, uint8_t>(114, 12),
				"a new grant replaces it");

		std::cout << "Mob attack passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
