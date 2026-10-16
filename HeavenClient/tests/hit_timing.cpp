#include "../Gameplay/Combat/HitTiming.h"
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
		using namespace ms;
		using hit_timing::target_delay;

		require(target_delay(Attack::Type::CLOSE, 0, 0) == 0, "the first melee target is hit at once");
		require(target_delay(Attack::Type::CLOSE, 1001005, 1) == 70, "each later melee target 70 ms after the last");
		require(target_delay(Attack::Type::CLOSE, 1001005, 3) == 210, "the fourth melee target 210 ms after the first");
		require(target_delay(Attack::Type::CLOSE, 1001005, 5) == 280, "melee targets wait at most 280 ms");

		require(target_delay(Attack::Type::MAGIC, 2201005, 0) == 0, "Thunderbolt's first target is hit at once");
		require(target_delay(Attack::Type::MAGIC, 2201005, 5) == 250, "Thunderbolt's targets are 50 ms apart, without a cap");
		require(target_delay(Attack::Type::MAGIC, 2221006, 2) == 200, "Chain Lightning's targets are 100 ms apart");
		require(target_delay(Attack::Type::MAGIC, 2001005, 1) == 0, "other spells hit at once");

		require(target_delay(Attack::Type::RANGED, 0, 2) == 0, "ranged hits are timed by their bullets");

		std::cout << "Hit timing passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
