#include "../Gameplay/Combat/Assassinate.h"

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
		using namespace ms::assassinate;

		require(refused(ID, false) && !refused(ID, true), "Assassinate goes only from Dark Sight");
		require(refused(DARK_SIGHT, true) && refused(4201005, true), "in Dark Sight nothing else goes");
		require(!refused(4201005, false) && !refused(0, true), "a regular attack always goes");

		require(multiplier(-1, 12) == 0, "never in Dark Sight, nothing");
		require(multiplier(0, 12) == 1 && multiplier(2999, 12) == 1, "under 3 seconds, once");
		require(multiplier(3000, 12) == 2 && multiplier(6000, 12) == 3 && multiplier(9000, 12) == 4,
				"once more every 3 seconds");
		require(multiplier(12000, 12) == 5 && multiplier(60000, 12) == 5, "up to the skill's time");
		require(multiplier(60000, 6) == 3 && multiplier(60000, 9) == 4, "lower levels build up less");

		std::cout << "assassinate tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}

	return 0;
}
