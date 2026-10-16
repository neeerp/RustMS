#include "../Character/Look/AttackStances.h"
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
		using namespace ms;
		using namespace ms::Stance;
		using attack_stances::get;

		using List = std::vector<Stance::Id>;

		require(get(1, false) == List{SWINGO1, SWINGO2, SWINGO3, STABO1, STABO2}, "one-handed weapons swing and stab");
		require(get(2, false) == List{SWINGT2, SWINGP1, SWINGP2, STABT1, STABT2}, "spears and polearms have five attacks");
		require(get(6, false) == List{SWINGO2}, "wands and staves swing once");
		require(get(8, false) == List{STABO1, STABO2}, "knuckles stab");

		require(get(3, true) == List{SHOOT1} && get(4, true) == List{SHOOT2}, "bows and crossbows shoot");
		require(get(7, true) == List{SWINGO1, SWINGO2, SWINGO3}, "claws throw with any of three swings");
		require(get(9, true) == List{SHOT}, "guns shoot");

		require(get(3, false) == List{SWINGT1, SWINGT3}, "bows swing without arrows");
		require(get(7, false) == List{STABO1, STABO2}, "claws stab without stars");
		require(get(9, false) == List{SWINGT1, SWINGT2, SWINGT2}, "guns mostly swing T2 without bullets");

		require(get(1, true).empty() && get(10, false).empty(), "other weapons don't shoot");

		std::cout << "Attack stances passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
