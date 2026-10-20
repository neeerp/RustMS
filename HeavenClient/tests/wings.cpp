#include "../Gameplay/Combat/Wings.h"

#include <cassert>
#include <cmath>

using namespace ms;

namespace
{
	void test_fall_speed()
	{
		// Level 1's x of 280 glides at 155.4 px/s and level 10's 100 at 55.5.
		assert(std::abs(wings::fall_speed(280) - 155.4) < 1e-9);
		assert(std::abs(wings::fall_speed(100) - 55.5) < 1e-9);
	}

	void test_may_cast()
	{
		assert(wings::may_cast(true, false, false));
		assert(wings::may_cast(false, true, false));
		assert(!wings::may_cast(false, false, false));
		assert(!wings::may_cast(true, false, true));
	}

	void test_glide()
	{
		assert(wings::gliding(true, 1.0));
		assert(!wings::gliding(true, 0.0));
		assert(!wings::gliding(true, -1.0));
		assert(!wings::gliding(false, 1.0));

		assert(wings::expired(0, true));
		assert(!wings::expired(1, true));
		assert(!wings::expired(-4000, false));
	}
}

int main()
{
	assert(wings::is_wings(5201005) && !wings::is_wings(5201006));
	test_fall_speed();
	test_may_cast();
	test_glide();

	return 0;
}
