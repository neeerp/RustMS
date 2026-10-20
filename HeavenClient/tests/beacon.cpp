#include "../Character/Beacon.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

using namespace ms::beacon;

int main()
{
	try
	{
		require(cast(HOMING_BEACON, 1) == BULLSEYE, "Bullseye fires in the beacon's place");
		require(cast(HOMING_BEACON, 0) == HOMING_BEACON, "not before it is learned");
		require(cast(5001001, 1) == 5001001, "nor for another skill");

		Mark mark{ HOMING_BEACON, 1000123 };
		require(guides(0, mark) && guides(2001004, mark), "shots and spells look for the mark");
		require(!guides(HOMING_BEACON, mark) && !guides(BULLSEYE, mark), "but not the beacons");
		require(!guides(0, Mark{}), "nor without a mark");

		require(bullseye(100.0, 20, 1000123, 1000123) == 120.0, "x percent more on the mark");
		require(bullseye(100.0, 20, 1000123, 1000124) == 100.0, "not on another mob");
		require(bullseye(100.0, 0, 1000123, 1000123) == 100.0, "nor without Bullseye");
		require(bullseye(100.0, 20, 0, 0) == 100.0, "nor without a mark");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	std::cout << "Beacon passed\n";
	return 0;
}
