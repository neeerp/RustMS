#include "../IO/UITypes/EnergyBarLayout.h"
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
		using namespace ms::energy_bar;

		require(fill(0) == 0 && fill(-5) == 0, "an empty gauge has no fill");
		require(fill(75) == 0, "one Energy Charge 1 hit is under a column");
		require(fill(129) == 1 && fill(5000) == 39, "a column for each 78th, truncated");
		require(fill(9999) == 77 && fill(FULL) == 78, "only a full gauge fills all 78");

		require(cap(2) && cap(79) && !cap(3) && !cap(78), "the caps are the fill's ends");

		std::cout << "Energy bar layout passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
