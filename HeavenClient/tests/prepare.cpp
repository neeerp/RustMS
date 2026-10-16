#include "../Gameplay/Combat/Prepare.h"
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
		using namespace ms::prepare;

		require(keydown(2121001) && keydown(3121004) && keydown(5101004) && keydown(22151001),
				"Big Bang, Hurricane, Corkscrew Blow and Evan's are held");
		require(!keydown(2111002) && !keydown(4211001), "Explosion and Chakra are timed, not held");
		require(repeats(5221004) && !repeats(3221001), "Rapid Fire keeps firing; Piercing Arrow charges");

		require(charge_cap(2321001) == 1000 && charge_cap(3221001) == 1000, "a charge fills in a second");
		require(charge_cap(3121004) == 2000 && charge_cap(13111002) == 2000, "two for those that keep firing");
		require(charge_cap(2111002) == 0, "Explosion has none");

		require(sends_charge(3121004, false) && sends_charge(5101004, false), "held shots and swings send it");
		require(sends_charge(2221001, true) && !sends_charge(2111002, true), "of the spells, Big Bang does");
		require(!sends_charge(1001004, false), "others don't");

		require(release_charge(2121001, 5) == 30, "a tap counts as 30 ms");
		require(release_charge(2121001, 640) == 640, "a charge is how long it was held");
		require(release_charge(2121001, 5000) == 1000, "up to its cap");

		require(charge_percent(2121001, 30) == 12, "a tap does 12%");
		require(charge_percent(2121001, 640) == 67, "the fraction is cut down");
		require(charge_percent(2121001, 1000) == 100, "a full charge does it all");
		require(charge_percent(3121004, 0) == 100, "an attack with no charge does it all");
		require(charge_percent(1001004, 500) == 100, "as does a skill that can't be charged");

		require(gauge(2121001, 0) == 2 && gauge(2121001, 14) == 2 && gauge(2121001, 15) == 3,
				"the gauge grows a pixel every 15 ms");
		require(gauge(2121001, 1000) == 68 && gauge(2121001, 1005) == 69 && gauge(2121001, 9000) == 69,
				"up to 69");
		require(gauge(3121004, 60) == 4, "every 30 ms for a 2 s cap");
		require(shows_gauge(2121001) && shows_gauge(1121001), "Big Bang and Monster Magnet show it");
		require(!shows_gauge(5221004) && !shows_gauge(22121000), "Rapid Fire and Evan's don't");

		std::cout << "Prepare passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
