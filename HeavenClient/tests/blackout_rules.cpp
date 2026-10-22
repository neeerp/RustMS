#include "../Gameplay/BlackoutRules.h"
#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool near(float a, float b)
{
	return std::fabs(a - b) < 1e-4f;
}

int main()
{
	try
	{
		using namespace ms::blackout;

		require(alpha(-1) == 0.0f && alpha(0) == 0.0f, "clear at first");
		require(near(alpha(360), 0.5f), "half in");
		require(alpha(720) == 1.0f && alpha(3919) == 1.0f, "black for 3200 ms");
		require(near(alpha(4180), 0.5f), "half out");
		require(alpha(4440) == 0.0f && alpha(10000) == 0.0f, "gone after 4440 ms");

		require(layers(1009, true) == 2 && layers(10001009, true) == 2 && layers(20011009, true) == 2,
				"one's own Bamboo Rain lays two");
		require(layers(1009, false) == 1, "someone else's one");
		require(layers(1020, true) == 1 && layers(20001020, true) == 1, "Rage of Pharaoh one");
		require(layers(20011020, true) == 0 && layers(20011020, false) == 1, "the Evan copy only for others");
		require(layers(1010, true) == 0 && layers(1001004, true) == 0, "nothing else");

		// Two layers at once are darker in the fades, and end together.
		State state;
		state.add(1000, 2);
		require(near(state.update(1360), 0.75f), "two halves");
		require(state.update(2000) == 1.0f, "black");
		require(state.update(5440) == 0.0f && state.starts.empty(), "both gone");

		state.add(0, 1);
		state.add(4000, 1);
		require(near(state.update(4000), 1.0f - 80.0f / 520) && state.starts.size() == 2, "a second cast overlaps");
		require(near(state.update(4440), 440.0f / 720) && state.starts.size() == 1, "the first ends, the second fades in");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "blackout rules passed" << std::endl;
	return 0;
}
