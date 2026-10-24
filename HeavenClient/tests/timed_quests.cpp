#include "../Character/TimedQuests.h"
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
		using namespace ms::timed_quests;

		std::vector<Clock> clocks;
		add(clocks, 3458, 1800000, 1800000);
		add(clocks, 3951, 1800000, 60000);
		add(clocks, 2262, 1800000, 1800000);
		require(clocks[0].x == 700 && clocks[0].y == 50, "the first clock is at (700, 50)");
		require(clocks[1].y == 100 && clocks[2].y == 150, "each next one 50 px under the last");

		require(remove(clocks, 3951), "the quest's clock comes down");
		require(!remove(clocks, 3951), "only once");
		require(clocks[1].quest == 2262 && clocks[1].y == 150, "the others stay where they were");
		add(clocks, 3951, 1800000, 60000);
		require(clocks[2].y == 200, "a new one goes under the last, not into the gap");

		add(clocks, 3458, 1800000, 1800000);
		require(clocks.size() == 4, "v83 doesn't look for a clock already up");

		require(clocks[3].serial != clocks[0].serial, "each clock is told apart");

		Clock full = { 1, 1800000, 1800000, 0, 0, 0 };
		require(segments(full) == 49, "a full gauge has its 48 steps and the cap");
		Clock half = { 1, 1800000, 900000, 0, 0, 0 };
		require(segments(half) == 25, "half the time, half the steps");
		Clock none = { 1, 1800000, 0, 0, 0, 0 };
		require(segments(none) == 1, "at 0 one step is left");

		require(frame(full, 16) == 0, "the first frame at the start");
		require(frame(half, 16) == 7, "frames spread over the whole time");
		require(frame(none, 16) == 14, "the last frame is kept for taking the clock down");

		std::vector<Clock> running;
		add(running, 1, 40000, 1500);
		tick(running, 1000);
		require(running[0].remaining == 500, "the time goes on");
		tick(running, 1000);
		require(running[0].remaining == 0, "and stops at 0");
		reset(running, 1);
		require(running[0].remaining == 40000, "mode 16 gives the whole time again");

		require(time_left(3723000) == "Time Left 1:2:3", "hours, minutes and seconds, unpadded");
		require(time_left(59999) == "Time Left 0:0:59", "whole seconds");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';

		return 1;
	}

	return 0;
}
