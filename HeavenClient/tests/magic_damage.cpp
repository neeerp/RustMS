#include "../Gameplay/Combat/MagicDamage.h"
#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool near(double a, double b)
{
	return std::fabs(a - b) < 1e-6;
}

int main()
{
	try
	{
		using namespace ms::magic_damage;

		// Monster attributes: a letter and a digit for each element.
		Attributes mob = attributes("F2I3");
		require(mob[2] == STRONG && mob[1] == WEAK, "fire is strong and ice weak against F2I3");
		require(mob[0] == NORMAL && mob[3] == NORMAL, "other elements are normal");
		require(attributes("H3")[5] == WEAK, "H is holy");
		require(element('f') == 2 && element('S') == 4 && element('d') == 6, "skills name elements in lower case");

		// Magic is INT plus magic attack, at most 1999.
		require(magic(100, 30) == 130, "INT counts toward magic");
		require(magic(1500, 800) == 1999, "magic stops at 1999");

		// Accuracy drops against monsters above the player, and not below.
		require(near(accuracy(100, 4, 15, 20), 5000.0 / 305), "five levels above");
		require(near(accuracy(100, 4, 20, 15), 5000.0 / 255), "a lower monster is like an even one");
		require(near(accuracy(109, 9, 20, 20), 5000.0 / 255), "INT and LUK count in tens");

		// The hit roll runs from half the accuracy to 120% of it.
		double even = accuracy(100, 4, 20, 20);
		require(!hits(even, 20, 0.0), "the low roll misses a monster with 20 avoid");
		require(hits(even, 20, 0.99), "the high roll hits it");
		require(hits(even, 9, 0.0), "every roll hits a monster with 9 avoid");
		require(hits(1000.0, 5000, 0.99), "avoid counts as at most 999");

		// Energy Bolt at level 20 (mad 55, mastery 10) with 130 magic and 100 INT.
		require(near(base(130, 100, 10, 55, 0.0), 186.18138), "the lowest roll uses 54% of magic");
		require(near(base(130, 100, 10, 55, 1.0), 294.71838), "the highest roll uses all of it");

		// Attributes scale the hit.
		require(near(elemental(100.0, 2101004, 2, mob), 50.0), "Fire Arrow on a strong monster does half");
		require(near(elemental(100.0, 2201004, 1, mob), 150.0), "Cold Beam on a weak monster does 150%");
		require(near(elemental(100.0, 2001004, 0, mob), 100.0), "Energy Bolt has no element");
		require(near(elemental(100.0, 2001004, 0, attributes("P1")), 0.0), "physical immunity stops it");
		require(near(elemental(100.0, 2111006, 0, attributes("F3S1")), 75.0),
				"Element Composition hits half as fire and half as poison");
		require(near(elemental(100.0, 2211006, 0, attributes("I2L3")), 100.0),
				"and half as ice and half as lightning");

		// Magic defense takes 50% to 60% of itself off.
		require(near(defend(186.18138, 30, 0.0), 171.18138), "the low defense roll takes 15");
		require(near(defend(294.71838, 30, 1.0), 276.71838), "the high one takes 18");
		require(finish(171.18138) == 171, "the number shown rounds down");
		require(finish(0.0) == 1 && finish(-40.0) == 1, "a hit shows at least 1");
		require(finish(250000.0) == 199999, "and at most 199999");

		std::cout << "Magic damage passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
