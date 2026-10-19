#include "../IO/ScreenshotRules.h"
#include <iostream>
#include <set>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::Screenshot;

		Throttle throttle;
		require(throttle.press(5000, true) == Press::TAKE, "the first press takes a shot");
		require(throttle.press(5400, true) == Press::TAKE, "a second one soon after does too");
		require(throttle.press(5800, true) == Press::TAKE, "and a third");
		require(throttle.press(6200, true) == Press::REFUSE, "the fourth within a second is refused");
		require(throttle.press(6600, true) == Press::IGNORE, "later ones do nothing");
		require(throttle.press(7599, true) == Press::IGNORE, "999 ms after the last still counts");
		require(throttle.press(8599, true) == Press::TAKE, "a full second after the last starts over");

		Throttle outside;
		for (int64_t now = 100; now < 1000; now += 100)
			require(outside.press(now, false) == Press::TAKE, "without the status bar every press takes one");

		require(outside.press(1000, true) == Press::TAKE, "entering the game counts from one");

		require(file_name(0) == "Maple0000.jpg", "names have four digits");
		require(file_name(12345) == "Maple12345.jpg", "and grow past them");

		auto first_free_of = [](int32_t count)
		{
			return first_free([count](int32_t index) { return index < count; });
		};

		for (int32_t count : { 0, 1, 2, 3, 4, 7, 8, 100, 1023, 1024 })
			require(first_free_of(count) == count, "the next index follows a run of shots");

		// A gap the probes or the bisection land on is reused.
		std::set<int32_t> taken = { 0, 1, 2, 4, 5, 6, 7, 8 };
		require(first_free([&](int32_t index) { return taken.count(index) > 0; }) == 3, "a gap is filled");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
