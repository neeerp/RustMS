#include "../Character/Look/PetFollow.h"
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
		using namespace ms::petfollow;

		require(role(false, false, 0) == 0, "a lone pet has role 0");
		require(role(true, false, 0) == 1 && role(true, false, 1) == 2, "two pets have roles 1 and 2");
		require(role(true, true, 0) == 5 && role(true, true, 1) == 3 && role(true, true, 2) == 4,
				"three pets have roles 5, 3 and 4");

		auto is = [](Goal g, int32_t x, int32_t radius) { return g.x == x && g.radius == radius; };

		require(is(goal(0, 1000, true), 1000, 80) && is(goal(0, 1000, false), 1000, 80), "role 0 heads for its owner");
		require(is(goal(1, 1000, true), 840, 80) && is(goal(1, 1000, false), 1000, 80),
				"role 1 goes ahead of an owner facing left");
		require(is(goal(2, 1000, true), 1000, 80) && is(goal(2, 1000, false), 1160, 80),
				"role 2 goes ahead of an owner facing right");
		require(is(goal(3, 1000, true), 840, 80) && is(goal(3, 1000, false), 960, 70),
				"role 3 goes ahead, or 40 px behind");
		require(is(goal(4, 1000, true), 1040, 70) && is(goal(4, 1000, false), 1160, 80),
				"role 4 goes 40 px behind, or ahead");
		require(is(goal(5, 1000, true), 1000, 60) && is(goal(5, 1000, false), 1000, 60), "role 5 stays close");

		require(turn_distance(false) == 80 && turn_distance(true) == 170, "it turns past 80 px, 170 with a Binocular");

		Goal g{ 1000, 80 };

		require(walk(79, g, 80, false, false, 1, 921) == 0, "nearer than 80 px it stops");
		require(walk(80, g, 80, false, true, 0, 920) == 0, "exactly 80 px away a standing pet stays");
		require(walk(81, g, 80, false, true, 0, 919) == 1, "a standing pet farther than 80 px starts toward its spot");
		require(walk(81, g, 80, false, true, 0, 1081) == -1, "and to the left when its spot is left");
		require(walk(80, g, 80, false, false, -1, 920) == -1, "a walking pet keeps its way within 80 px");
		require(walk(80, g, 80, false, false, 0, 920) == 0, "a moving pet that stopped doesn't start until past the turn");
		require(walk(81, g, 80, false, false, -1, 919) == 1, "past the turn it heads back");
		require(walk(171, g, 170, false, false, -1, 829) == 1 && walk(170, g, 170, false, false, -1, 830) == -1,
				"with a Binocular it turns past 170 px");
		require(walk(80, g, 80, true, false, -1, 920) == 1, "sweeping, it heads for its spot whenever it may walk");
		require(walk(80, Goal{ 920, 80 }, 80, true, false, -1, 920) == 1, "level with its spot it heads right");

		require(hops(1, false, false, true, 0) && hops(1, false, false, true, 1000) && hops(1, false, false, true, 32000),
				"a hop comes at one roll in 1000");
		require(!hops(1, false, false, true, 999), "not at other rolls");
		require(!hops(0, false, false, true, 0), "never for a lone pet");
		require(!hops(1, true, false, true, 0), "nor a sweeping one");
		require(!hops(1, false, true, true, 0), "nor one already running");
		require(!hops(1, false, false, false, 0), "nor one that didn't move");
		require(dashes(2, false, false, true, 700) && !dashes(2, false, false, true, 699), "a dash comes at one in 700");
		require(!dashes(0, false, false, true, 0), "never for a lone pet");

		require(!run_over(700) && run_over(701), "a run lasts 700 ms");

		require(jumps_up(1, 2, 100, 200, 0, 500, 250), "under its owner's platforms it jumps");
		require(!jumps_up(2, 2, 100, 200, 0, 500, 250), "not on the owner's own platforms");
		require(!jumps_up(1, 2, 200, 200, 0, 500, 250), "nor when the owner isn't above it");
		require(!jumps_up(1, 2, 100, 200, 250, 500, 250) && !jumps_up(1, 2, 100, 200, 0, 250, 250),
				"nor at either end of their reach");

		require(jump_direction(100, 200, false) == -1 && jump_direction(300, 200, true) == 1, "it jumps toward its owner");
		require(jump_direction(200, 200, true) == -1 && jump_direction(200, 200, false) == 1,
				"right under them, the way they face");

		int32_t elapsed = 0;

		require(!step(elapsed, 8) && !step(elapsed, 8) && !step(elapsed, 8), "no step before 30 ms");
		require(step(elapsed, 8) && elapsed == 2, "a step at 32 ms keeps the 2 ms over");
	}
	catch (const std::exception& e)
	{
		std::cerr << "pet_follow test failed: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "pet_follow tests passed" << std::endl;
	return 0;
}
