#include "../Gameplay/MapleMap/ReactorRules.h"
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
		using namespace ms::reactor_rules;
		using Types = std::vector<int32_t>;

		int32_t right_ground = flags(false, true);
		int32_t left_ground = flags(true, true);
		int32_t right_air = flags(false, false);
		int32_t left_air = flags(true, false);

		require(right_ground == 2 && left_ground == 3 && right_air == 0 && left_air == 1,
				"facing left is bit 0, standing bit 1");

		require(rank(right_ground, 0) == 1 && rank(right_air, 0) == 2, "any swing takes type 0");
		require(rank(right_ground, 1) == 0 && rank(right_air, 1) == 1, "type 1 one facing right");
		require(rank(left_ground, 1) == -1 && rank(left_air, 1) == -1, "but not left");
		require(rank(left_ground, 2) == 0 && rank(left_air, 2) == 1, "type 2 one facing left");
		require(rank(right_ground, 2) == -1, "but not right");
		require(rank(right_air, 3) == 0 && rank(left_air, 4) == 0, "types 3 and 4 a jump");
		require(rank(right_ground, 3) == -1 && rank(left_air, 3) == -1 && rank(right_air, 4) == -1,
				"facing their way");
		require(rank(right_ground, 5) == -1 && rank(right_ground, 100) == -1, "no other type");

		require(swingable({ 5, 0 }) && swingable({ 4 }), "a state with a type from 0 to 4 takes a swing");
		require(!swingable({}) && !swingable({ 5, 100, 101 }), "one without doesn't");

		require(pick_event({ 0, 0 }, right_ground) == 0, "the first of a rank");
		require(pick_event({ 0, 1 }, right_ground) == 1, "a better rank wins");
		require(pick_event({ 0, 1 }, left_ground) == 0, "of those that suit");
		require(pick_event({ 1, 3 }, right_air) == 1, "a jump prefers type 3");
		require(pick_event({ 2, 1 }, right_air) == 1, "only the first best is taken");
		require(pick_event({ 5, 1 }, left_ground) == -1, "or none");

		Box range{ 100, -60, 180, 0 };
		Box behind{ 60, -40, 108, -6 };
		Box ahead{ 150, -40, 198, -6 };

		require(pick_target({}, range, 120, false) == -1, "nothing to hit");
		require(pick_target({ behind, ahead }, range, 120, false) == 1, "the first ahead");
		require(pick_target({ ahead, behind }, range, 120, false) == 0, "whatever order they came in");
		require(pick_target({ behind, Box{} }, range, 120, false) == 0, "else the last reached");
		require(pick_target({ Box{ 200, -40, 248, -6 } }, range, 120, false) == -1, "if any");
		require(pick_target({ Box{ 180, -40, 228, -6 } }, range, 120, false) == -1, "touching isn't reaching");
		require(pick_target({ Box{ 150, -40, 150, -6 } }, range, 120, false) == -1, "nor is an empty box");
		require(pick_target({ ahead, behind }, range, 120, true) == 1, "facing left, v83 takes the last");
		require(pick_target({ Box{ -174, -40, -126, -6 }, Box{ -284, -40, -236, -6 } }, Box{ -320, -60, -140, 0 }, -200,
							true) == 0, "unless the middle is left of -x, even behind the user");
		require(pick_target({ Box{ -3, -40, 0, -6 }, Box{ 0, -40, 10, -6 } }, Box{ -10, -60, 10, 0 }, -1, false) == 0,
				"halving rounds towards 0");

		std::cout << "Reactor rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
