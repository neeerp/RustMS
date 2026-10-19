#include "../IO/UITypes/Quiz.h"
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
		using namespace ms::quiz;

		require(seconds_left(30000) == 30 && seconds_left(29999) == 29, "whole seconds, rounded down");
		require(seconds_left(999) == 0 && seconds_left(0) == 0 && seconds_left(-5) == 0,
			"the time is up with less than a second left");

		require(countdown(0) == std::array<int32_t, 4>{ 0, 0, 0, 0 }, "nothing left");
		require(countdown(59) == std::array<int32_t, 4>{ 0, 0, 5, 9 }, "under a minute");
		require(countdown(754) == std::array<int32_t, 4>{ 1, 2, 3, 4 }, "minutes and seconds");

		require(length_problem("abcd", 2, 5).empty(), "twice the fewest letters is enough");
		require(length_problem("abc", 2, 5) == "You must enter atleast 4 letters. (Korean)",
			"too short gives the doubled least");
		require(length_problem("abcdefghij", 2, 5).empty(), "twice the most letters is allowed");
		require(length_problem("abcdefghijk", 2, 5) == "You must enter less than 10 letters. (Korean)",
			"too long gives the doubled most");

		require(tick_volume(120, 300) == 60, "two minutes of five left");
		require(tick_volume(0, 300) == 100 && tick_volume(0, 0) == 100, "loudest at the end");
		require(tick_volume(30, 60) == 50, "half the time left");

		std::cout << "Quiz passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
