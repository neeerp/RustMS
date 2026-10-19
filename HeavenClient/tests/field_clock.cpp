#include "../Gameplay/MapleMap/ClockRules.h"
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
		using namespace ms::countdown;

		require(seconds_left(10000) == 10 && seconds_left(9999) == 9, "whole seconds, rounded down");
		require(seconds_left(-1) == 0 && seconds_left(-1000) == -1, "rounded toward zero past the end");

		auto digits = [](int32_t seconds) { return face(seconds).digits; };

		require(!face(3599).hours && face(3600).hours, "hours from one on");
		require(digits(754) == std::array<int32_t, 4>{ 1, 2, 3, 4 }, "minutes and seconds");
		require(digits(300) == std::array<int32_t, 4>{ -1, 5, 0, 0 }, "a first 0 is left out");
		require(digits(0) == std::array<int32_t, 4>{ -1, 0, 0, 0 } && digits(-7) == digits(0),
			"below 1 shows 0");
		require(digits(7200) == std::array<int32_t, 4>{ -1, 2, 0, 0 }, "two hours");
		require(digits(5940 + 3600 * 10) == std::array<int32_t, 4>{ 1, 1, 3, 9 }, "hours and minutes");

		require(stays_on(109020001) && stays_on(209000000), "event maps");
		require(!stays_on(109050000) && !stays_on(100000000) && !stays_on(910000000),
			"not their 5s or other maps");

		std::cout << "Field clock passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
