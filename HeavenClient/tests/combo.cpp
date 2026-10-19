#include "../Gameplay/Combat/Combo.h"

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
		using namespace ms::combo;

		require(finisher(1111003) && finisher(1111006) && finisher(11111003), "Panic and Coma finish");
		require(!finisher(1111002) && !finisher(1111008) && !finisher(0), "the rest don't");

		// Combo Attack 30, whose damage is 120.
		require(percent(false, 30, 120, -1) == 0, "no buff, no change");
		require(percent(false, 30, 120, 0) == 100, "no orbs");
		require(percent(false, 30, 120, 1) == 120, "one orb is the base");
		require(percent(false, 30, 120, 2) == 125, "two orbs");
		require(percent(false, 30, 120, 5) == 140, "five orbs");
		require(percent(false, 1, 100, 5) == 100, "level 1 gains nothing more");
		require(percent(false, 30, 150, 10) == 190, "ten orbs with Advanced Combo");
		require(percent(false, 30, 150, 11) == 100, "past ten");

		require(percent(true, 30, 120, 0) == 100, "a finisher without orbs");
		require(percent(true, 30, 120, -1) == 100, "a finisher without the buff");
		require(percent(true, 30, 120, 2) == 150, "a finisher with two");
		require(percent(true, 30, 120, 3) == 200, "a finisher with three");
		require(percent(true, 30, 120, 5) == 350, "a finisher with five");
		require(percent(true, 19, 120, 4) == 30 + 120 * 18 / 29 + 120, "the steps scale with the level");
		require(percent(true, 30, 150, 10) == 470, "a finisher with ten");

		Line hero = line(112);
		Line dawn = line(1111);
		require(hero.combo == 1111002 && hero.advanced == 1120003 && hero.last == 30, "a Hero's");
		require(dawn.combo == 11111001 && dawn.advanced == 11110005 && dawn.last == 20, "a Dawn Warrior's");
		require(base_skill(hero, 30, 1) == 1120003, "Advanced Combo once Combo Attack is maxed");
		require(base_skill(hero, 29, 30) == 1111002, "Combo Attack before");
		require(base_skill(hero, 30, 0) == 1111002, "Combo Attack without Advanced Combo");
	} catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
