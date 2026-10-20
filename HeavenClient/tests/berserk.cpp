#include "../Character/Berserk.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

using namespace ms::berserk;

int main()
{
	try
	{
		// Level 30: x 50, damage 100.
		require(damage(DARK_KNIGHT, 30, 500, 1000, 50, 100) == 100, "berserk at half HP");
		require(damage(DARK_KNIGHT, 30, 501, 1000, 50, 100) == 0, "not above it");
		require(damage(131, 30, 100, 1000, 50, 100) == 0, "only a Dark Knight");
		require(damage(DARK_KNIGHT, 0, 100, 1000, 50, 100) == 0, "who learned it");
		require(damage(DARK_KNIGHT, 1, 209, 1000, 21, 32) == 32, "x is a percent of max HP");

		require(changed(0, 100) && changed(100, 0), "turning on or off is told");
		require(!changed(100, 100) && !changed(0, 0), "staying on or off is not");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	return 0;
}
