#include "../Gameplay/Combat/Dash.h"

#include <cassert>

using namespace ms;

namespace
{
	constexpr int32_t LEFT = 1;
	constexpr int32_t RIGHT = 2;
	constexpr int32_t OTHER = 0;

	void test_skill()
	{
		assert(dash::skill(500, 0) == dash::ID);
		assert(dash::skill(510, 1902000) == dash::ID);
		assert(dash::skill(1500, 0) == dash::DAWN_ID);
		assert(dash::is_dash(5001005) && dash::is_dash(15001003) && !dash::is_dash(5001002));

		// On an event vehicle it is Space Dash, the Noblesse's for the Cygnus line; the Battleship isn't one.
		assert(dash::skill(0, 1932002) == dash::SPACE_ID);
		assert(dash::skill(2000, 1932003) == dash::SPACE_ID);
		assert(dash::skill(1000, 1932010) == dash::DAWN_SPACE_ID);
		assert(dash::skill(520, 1932000) == dash::ID);
		assert(dash::is_dash(1014) && dash::is_dash(10001015) && !dash::is_dash(1013));
	}

	void test_ride_bonus()
	{
		// Space Dash's 50 adds to an event vehicle's speed, up to 190, and its 1 to the jump, up to 123.
		assert(dash::ride_speed(1932001, 100, 50) == 150);
		assert(dash::ride_speed(1932005, 170, 50) == 190);
		assert(dash::ride_speed(1932001, 100, 0) == 100);
		assert(dash::ride_jump(1932003, 100, 1) == 101);
		assert(dash::ride_jump(1932003, 123, 1) == 123);
		// A tamed monster or the Battleship keeps its own.
		assert(dash::ride_speed(1902000, 150, 50) == 150);
		assert(dash::ride_jump(1932000, 100, 1) == 100);
	}

	void test_bonus()
	{
		// Haste 20 and Dash 30 make 30, Dash 12 alone 12, and a Slow keeps its own.
		assert(dash::speed_bonus(20, 30) == 10);
		assert(dash::speed_bonus(0, 12) == 12);
		assert(dash::speed_bonus(40, 30) == 0);
		assert(dash::speed_bonus(-10, 30) == 0);
		assert(dash::jump_bonus(0, 10) == 10);
		assert(dash::jump_bonus(15, 10) == 0);
	}

	void test_double_tap()
	{
		dash::Command command;
		assert(!command.feed(RIGHT, true, 1000, false));
		assert(!command.feed(RIGHT, false, 1100, false));
		assert(command.feed(RIGHT, true, 1300, false));

		// The second press comes too late, but starts a new tap.
		dash::Command late;
		assert(!late.feed(LEFT, true, 1000, false));
		assert(!late.feed(LEFT, false, 1100, false));
		assert(!late.feed(LEFT, true, 1301, false));
		assert(!late.feed(LEFT, false, 1350, false));
		assert(late.feed(LEFT, true, 1400, false));

		// Another arrow or key between drops it.
		dash::Command mixed;
		assert(!mixed.feed(LEFT, true, 0, false));
		assert(!mixed.feed(LEFT, false, 50, false));
		assert(!mixed.feed(RIGHT, true, 100, false));
		assert(!mixed.feed(RIGHT, false, 120, false));
		assert(!mixed.feed(OTHER, true, 150, false));
		assert(!mixed.feed(RIGHT, true, 200, false));

		// A held key repeating its press isn't a second press.
		dash::Command held;
		assert(!held.feed(RIGHT, true, 0, false));
		assert(!held.feed(RIGHT, true, 50, false));
		assert(!held.feed(RIGHT, false, 60, false));
		assert(held.feed(RIGHT, true, 100, false));

		// None starts while dashing.
		dash::Command dashing;
		assert(!dashing.feed(RIGHT, true, 0, true));
		assert(!dashing.feed(RIGHT, false, 50, true));
		assert(!dashing.feed(RIGHT, true, 100, true));
	}
}

int main()
{
	test_skill();
	test_bonus();
	test_ride_bonus();
	test_double_tap();

	return 0;
}
