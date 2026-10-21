#include "../Character/AutoPotions.h"
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
		using namespace ms::auto_potion;

		require(below(199, 0, 1000, 20), "a stat under the alert gives");
		require(!below(200, 0, 1000, 20), "a stat at the alert doesn't");
		require(below(500, 301, 1000, 20), "the hit not yet taken counts");
		require(!below(0, 0, 1000, 0), "an alert of 0 never gives");
		require(!below(10, 0, 0, 20), "nor does a stat with no maximum");
		require(below(10, 500, 1000, 20), "a hit past the stat still gives");
		require(!below(30000, 0, 30000, 100), "a full stat is never under 100%");
		require(below(29999, 0, 30000, 100), "but anything less is");

		require(pets_complain(1) && pets_complain(3), "the pets complain three times in a row");
		require(!pets_complain(4), "and then keep quiet");

		require(change(0, false, false, 2000000) == Change::CLEAR, "0 clears, pouch or not");
		require(change(2000000, false, true, 0) == Change::NONE, "without the pouch nothing happens");
		require(change(2010000, true, false, 0) == Change::REFUSED, "an item that heals nothing is refused");
		require(change(2000000, true, true, 2000000) == Change::SAME, "the same potion stays");
		require(change(2000001, true, true, 2000000) == Change::SET, "another potion is set");

		std::cout << "Auto potions passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
