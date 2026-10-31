#include "../Gameplay/Combat/AranCombo.h"

#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::aran_combo;

		require(ability(2000) == 20000017 && ability(2100) == 21000000 && ability(2112) == 21000000,
				"a beginner's Combo Ability is its own");

		require(needs(21100004) == 30 && needs(21100005) == 30, "Combo Smash and Combo Drain need 30");
		require(needs(21110004) == 100, "Combo Penril 100");
		require(needs(21120006) == 200 && needs(21120007) == 200, "Combo Tempest and Combo Barrier 200");
		require(needs(21100002) == 0 && needs(0) == 0, "the rest nothing");

		require(counts(true, 1, 1, POLEARM), "a polearm hit of the player's counts");
		require(!counts(false, 1, 1, POLEARM), "not someone else's");
		require(!counts(true, 0, 1, POLEARM), "not a miss");
		require(!counts(true, 1, 0, POLEARM), "not without Combo Ability");
		require(!counts(true, 1, 1, 143), "not with a spear");

		require(!idle(5, 1000, 4000) && idle(5, 1000, 4001), "a count goes once 3 seconds have passed since it came");
		require(!idle(0, 1000, 9000), "and none has nothing to go");

		require(tier(1) == 0 && tier(29) == 0 && tier(30) == 1 && tier(99) == 1, "the first tiers break at 30");
		require(tier(100) == 2 && tier(199) == 2 && tier(200) == 3 && tier(30000) == 3, "and 100 and 200");

		require(digits(305) == std::vector<int32_t>({ 5, 0, 3 }), "the digits come the ones first");
		require(digits(0).empty() && digits(123456).size() == 5, "five at most");

		require(digit_y(0, 1) == 165, "a lone digit sits at 165");
		require(digit_y(0, 2) == 169 && digit_y(1, 2) == 165, "the first is always at 165");
		require(digit_y(0, 3) == 165 && digit_y(1, 3) == 169 && digit_y(2, 3) == 165, "and they alternate");

		require(bounce_height(31, 0) == 10 && bounce_height(31, 1) == 4, "the lowest digit bounces 10, the rest 4");
		require(bounce_height(30, 0) == 10 && bounce_height(30, 1) == 10, "and the zeros under it");
		require(bounce_height(100, 2) == 10 && bounce_height(140, 2) == 4, "up to the lowest that isn't 0");
		require(bounce_height(200, 0) == 0 && bounce_height(201, 0) == 0, "nothing bounces from 200");

		require(bounce(10, 0) == 0 && bounce(10, 25) == 5 && bounce(10, 50) == 10, "10 goes down evenly in 50 ms");
		require(bounce(10, 75) == 10 && bounce(10, 175) == 6 && bounce(10, 300) == 0, "and springs back by 300");
		require(bounce(4, 50) == 4 && bounce(4, 90) == 4 && bounce(4, 299) == 0, "4 too");
		require(bounce(0, 25) == 0, "no height, no bounce");

		require(attack(0, 10, 1) == 0 && attack(9, 10, 1) == 0 && attack(57, 10, 1) == 5, "a point for every ten");
		require(attack(300, 10, 1) == 10 && attack(300, 3, 2) == 6, "up to x tens");
		require(critical(2000) == 20000018 && critical(2110) == 21110000, "Combo Critical");
		require(critical(9, 5, 3, 5).rate == 10 && critical(9, 5, 3, 5).damage == 0, "10 percent before the first ten");
		require(critical(57, 5, 3, 5).rate == 25 && critical(57, 5, 3, 5).damage == 25, "y and damage for every ten");
		require(critical(300, 5, 3, 5).rate == 25 && critical(300, 5, 3, 5).damage == 25, "up to x tens");

		require(pushed(100, 10, false) == 10 && pushed(33, 60, false) == 19, "Smart Knockback keeps its percent");
		require(pushed(100, 10, true) == 100 && pushed(100, 0, false) == 100, "not of a boss, nor without it");

		Command thirty = command(30);
		require(thirty.index == 1 && thirty.first == 21100004 && thirty.second == 21100005, "30 frees Combo Smash and Drain");
		Command hundred = command(100);
		require(hundred.index == 2 && hundred.first == 21110004 && hundred.second == 21100005, "100 Combo Penril");
		Command two_hundred = command(200);
		require(two_hundred.index == 3 && two_hundred.first == 21120006 && two_hundred.second == 21120007,
				"200 Combo Tempest and Barrier");
		require(command(31).index == 0 && command(300).index == 0, "only those counts announce anything");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
