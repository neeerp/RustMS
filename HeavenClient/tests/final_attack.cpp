#include "../Gameplay/Combat/FinalAttack.h"

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
		using namespace ms::final_attack;

		// A third of the way from the hit at 300 ms to the end at 900.
		require(delay(300, 900) == 500, "it follows a third of the way from the hit to the end");
		require(delay(300, 301) == 300, "rounding down");

		require(weapon_type(1302000) == 30 && weapon_type(1442000) == 44 && weapon_type(1452000) == 45,
				"a weapon's type is its id's");
		require(swings(30) && swings(33) && swings(40) && swings(44), "swords, axes, blunt weapons, spears and polearms swing");
		require(!swings(34) && !swings(39) && !swings(45), "nothing else swings");
		require(shoots(45) && shoots(46) && shoots(47) && shoots(49), "bows, crossbows, claws and guns shoot");
		require(!shoots(48) && !shoots(44), "nothing else shoots");

		require(needs_buff(11101002) && needs_buff(13101002) && !needs_buff(1100002), "the Cygnus Knights' need a buff");

		require(slash_blast(1001005) && slash_blast(11001003) && !slash_blast(1001004), "Slash Blast");
		require(slash_blast_of(130) == 1001005 && slash_blast_of(1110) == 11001003, "a Dawn Warrior has their own");
		require(stretches(1001005) && stretches(1111006) && stretches(1211002) && stretches(11111003),
				"Slash Blast, Coma and Charged Blow stretch");
		require(!stretches(1001004) && !stretches(1100002), "the rest don't");

		require(fall_off(1000, 0) == 666 && fall_off(1000, 1) == 222 && fall_off(1000, 2) == 74,
				"each mob after Slash Blast takes a third of the last one's");
		require(fall_off(1000, 5) == 2 && fall_off(100, 5) == 0, "cut down to a whole number");
		require(fall_off(1000, 20) == 0, "past the table, nothing");

		std::cout << "Final Attack passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
