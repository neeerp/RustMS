#include "../Character/TouchDamage.h"
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
		using namespace ms::touch_damage;

		// A new beginner: 12 STR, 5 DEX, 4 INT, 4 LUK, and starter clothes for 7 W.DEF.
		int32_t beginner = defense(12, 5, 4, 4, 7);
		require(beginner == 20, "stats add hidden defense on top of the clothes");
		require(defense(12, 5, 4, 4, 0) == 15, "a beginner without clothes still has defense from stats");

		// A snail (attack 12) hits for 1 every time, as in the period footage.
		require(min_base(12) == 10, "a snail's base damage starts at 85% of its attack");
		for (int32_t base = min_base(12); base < 12; base++)
			require(damage(base, beginner, 0.0f) == 1, "a snail hits a beginner for 1");

		// A Shroom or Blue Snail (attack 17) hits a level 5 beginner for 1 or 2.
		for (int32_t base = min_base(17); base < 17; base++)
		{
			int32_t hit = damage(base, defense(12, 5, 4, 4, 0) - 1, 0.0f);
			require(hit >= 1 && hit <= 2, "a Shroom hits a beginner for 1 or 2");
		}

		// A Green Mushroom (attack 82) still hurts a level 15 magician in robes.
		int32_t magician = defense(4, 4, 60, 4, 10);
		require(magician == 32, "INT gives the least hidden defense");
		require(damage(min_base(82), magician, 0.0f) == 37, "the lowest Green Mushroom hit");
		require(damage(81, magician, 0.0f) == 49, "the highest Green Mushroom hit");

		// The same hit on a level 15 warrior in armor takes far less.
		int32_t warrior = defense(45, 25, 4, 4, 20);
		require(warrior == 67, "STR gives the most hidden defense");
		require(damage(81, warrior, 0.0f) == 14, "armor and STR soak most of the hit");

		require(damage(81, magician, 0.5f) == 25, "Achilles-like reductions apply after defense");
		require(damage(5, 100, 0.5f) == 1, "a hit never takes less than 1");

		std::cout << "Touch damage passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
