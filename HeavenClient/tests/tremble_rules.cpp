#include "../Gameplay/TrembleRules.h"
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
		using namespace ms::tremble;

		State light = set(30, false, 500, 0, 1000);
		require(light.start == 1500 && light.end == 3500 && light.factor == 0.92, "a light shake");
		State heavy = set(30, true, 0, 0, 1000);
		require(heavy.end == 2500 && heavy.factor == 0.85, "a heavy shake fades faster");
		State steady = set(10, false, 0, 4000, 0);
		require(steady.end == 6000 && steady.factor == 1.0, "an additional time keeps it steady");

		// Nothing before the delay, then within the force either way.
		require(step(light, 1500, 59, 0) == Offset{ 0, 0 } && light.force == 30, "waits out the delay");
		require(step(light, 1530, 59, 0) == Offset{ 29, -30 }, "-30 to 29");
		require(light.force == 30 * 0.92, "fades each tick");

		// A heavy shake falls under 1 in 21 ticks, a light one in 41.
		int heavy_ticks = 0;
		for (int32_t now = 1030; heavy.force > 0; now += TICK)
		{
			step(heavy, now, 0, 0);
			heavy_ticks++;
		}
		require(heavy_ticks == 21, "heavy");
		State fading = set(30, false, 0, 0, 0);
		int light_ticks = 0;
		for (int32_t now = 30; fading.force > 0; now += TICK)
		{
			step(fading, now, 0, 0);
			light_ticks++;
		}
		require(light_ticks == 41, "light");

		// A steady one stops at its end.
		require(step(steady, 5970, 0, 0) == Offset{ -10, -10 }, "steady");
		require(step(steady, 6000, 0, 0) == Offset{ 0, 0 } && steady.force == 0, "ends");

		// Shout shakes harder, and someone else's skill less with distance.
		require(skill_shakes(1111008, true) && skill_heavy(1111008), "Shout");
		require(skill_shakes(1311006, true) && !skill_heavy(1311006), "Dragon Roar");
		require(!skill_shakes(20011020, true) && skill_shakes(20011020, false), "only others' 20011020");
		require(!skill_shakes(1001004, true), "Power Strike");
		require(skill_force(1111008, 0) == 400.0 / 15 && skill_force(1311006, 0) == 16, "forces");
		require(skill_force(1311006, 300) == 4 && skill_force(1311006, 500) == 0, "distance");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
