#include "../Character/SecondaryStat.h"
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
		using namespace ms::secondary_stat;

		require(in_line(312, 311) && in_line(311, 311) && !in_line(310, 311), "a line and its later jobs");
		require(in_line(322, 300) && !in_line(412, 300), "a first job takes in every job of it");
		require(!in_line(1110, 111), "Cygnus are not of the adventurers' line");

		require(base_accuracy(100, 25, 10) == 25, "a warrior's DEX counts 0.8 and LUK 0.5");
		require(base_accuracy(312, 100, 10) == 63, "an archer's DEX counts 0.6 and LUK 0.3");
		require(base_accuracy(1400, 100, 10) == 63, "so does a Night Walker's");
		require(base_accuracy(500, 100, 10) == 63, "and a pirate's");
		require(base_accuracy(510, 100, 10) == 93, "a Brawler's DEX counts 0.9");

		require(base_avoid(0, 4, 5) == 3, "a quarter of DEX and half of LUK, each rounded down");
		require(base_avoid(510, 10, 5) == 17, "a Brawler's DEX counts one and a half");
		require(base_avoid(520, 20, 5) == 4, "a Gunslinger's an eighth");

		require(shield_mastery(111) == 1110001 && shield_mastery(112) == 1110001, "the Crusader's");
		require(shield_mastery(121) == 1210001 && shield_mastery(422) == 4210000, "the White Knight's and the Bandit's");
		require(shield_mastery(110) == 0 && shield_mastery(131) == 0 && shield_mastery(412) == 0, "none for the rest");
		require(shield_defence(25, 200) == 50 && shield_defence(17, 150) == 25, "x percent, rounded down");

		require(echo_bonus(103, 4) == 4 && echo_bonus(1000, 4) == 40, "Echo's percent, rounded down");
		require(echo_bonus(103, 0) == 0, "none without it");

		require(bare_hands_attack(500, 1) == 10 && bare_hands_attack(510, 20) == 24, "ten and 0.7 a level");
		require(bare_hands_attack(500, 30) == 31 && bare_hands_attack(520, 120) == 31, "at most 31");
		require(bare_hands_attack(1500, 10) == 17, "Thunder Breakers too");
		require(bare_hands_attack(100, 30) == 0 && bare_hands_attack(0, 30) == 0, "no one else");

		std::cout << "secondary stat tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";

		return 1;
	}

	return 0;
}
