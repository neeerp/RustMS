#include "../Gameplay/Combat/MonsterMagnet.h"
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
		using namespace ms::magnet;

		require(is_magnet(1121001) && is_magnet(1221001) && is_magnet(1321001), "the three Monster Magnets");
		require(!is_magnet(1121000) && !is_magnet(5201002), "nothing else");

		require(count(6, 30, 1000) == 1 && count(6, 1000, 1000) == 6, "one mob, up to its mob count when fully charged");
		require(count(6, 500, 1000) == 3 && count(6, 199, 1000) == 1 && count(6, 200, 1000) == 2,
				"the rest as it charges, rounded down");
		require(count(6, 0, 0) == 6, "without a cap, the whole count");
		require(count(16, 1000, 1000) > MAX_COUNT, "more than 15 isn't used");

		require(catches(false, true, 95, 94) && !catches(false, true, 95, 95), "caught at its prop");
		require(!catches(true, true, 100, 0), "never a boss");
		require(!catches(false, false, 100, 0), "nor a mob that can't move");

		require(start_speed(500, 100, 85) == -PULL_SPEED && start_speed(100, 500, 85) == PULL_SPEED,
				"it starts toward the user at 1000 px/s");
		require(start_speed(150, 100, 85) == 0.0 && start_speed(184, 100, 85) == 0.0 && start_speed(185, 100, 85) == -PULL_SPEED,
				"not at all within its reach");

		require(step_speed(-1000.0, 300, 100, 85) == -1000.0, "full speed while far");
		require(step_speed(-1000.0, 224, 100, 85) == -1000.0 / 3 && step_speed(-1000.0, 225, 100, 85) == -1000.0,
				"a third each step within 40 px past its reach");

		require(fly_speed(100, 200) == FLY_SPEED && fly_speed(200, 100) == -FLY_SPEED, "a flier rises or sinks toward the user");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "magnet tests passed\n";
	return 0;
}
