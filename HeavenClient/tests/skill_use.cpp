#include "../Gameplay/Combat/SkillUse.h"
#include <iostream>
#include <stdexcept>
#include <utility>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::skill_use;

		require(sends_position(2301004) && sends_position(1001003) && sends_position(2311001),
				"Bless, Iron Body and Dispel send where the user stands");
		require(!sends_position(2111004) && !sends_position(SHADOW_STARS) && !sends_position(2301002),
				"Seal, Shadow Stars and Heal don't");

		require(reach(2301004) == PARTY && reach(RESURRECTION) == PARTY, "Bless and Resurrection reach the party");
		require(reach(2111004) == MOBS && reach(1111007) == MOBS, "Seal and Armor Crash reach mobs");
		require(reach(DISPEL) == (PARTY | MOBS), "Dispel reaches both");
		require(reach(1001003) == SELF && reach(SHADOW_STARS) == SELF, "Iron Body and Shadow Stars only the user");

		require(party_bit(0) == 0x20 && party_bit(5) == 0x01, "the first slot is the highest bit");
		require(ALONE == 0x80, "the user alone is 0x80");

		require(reaches(2301004, true, false) && reaches(2301004, false, false), "a buff reaches the living");
		require(!reaches(2301004, false, true), "but not the dead");
		require(reaches(RESURRECTION, false, true), "Resurrection raises the dead");
		require(!reaches(RESURRECTION, true, true) && !reaches(RESURRECTION, false, false),
				"but never the user, nor the living");
		require(!sends(RESURRECTION, 0) && sends(RESURRECTION, 0x10) && sends(2301004, 0),
				"Resurrection with no one to raise isn't sent");
		require(sends(RESURRECTION, ALONE), "but alone it is");

		require(mob_limit(6, 2) == 4 && mob_limit(1, 1) == 0 && mob_limit(1, 3) == 0,
				"the party reached takes from the mob count");

		std::vector<std::pair<int32_t, int32_t>> use = {
			{ 2000000, 50 }, { 2070000, 0 }, { 2070000, 150 }, { 2070001, 100 }, { 2070000, 50 }
		};
		require(shadow_star(use, 200) == 2070000, "stacks of a star after the first add up");
		require(shadow_star(use, 201) == 0, "but not those before it, nor other stars");
		require(shadow_star(use, 100) == 2070000, "the first star that has enough is taken");

		std::vector<std::pair<int32_t, int32_t>> split = { { 2070000, 100 }, { 2070001, 200 } };
		require(shadow_star(split, 200) == 2070001, "a later star may have enough when the first doesn't");
		require(shadow_star(split, 0) == 0, "a level that needs none spends none");
		require(shadow_star(std::vector<std::pair<int32_t, int32_t>>{ { 2330000, 500 } }, 200) == 0,
				"bullets aren't stars");

		require(star_lost(1, false) && star_lost(7, false), "the buff's star gone from USE is lost");
		require(!star_lost(1, true), "a star still held isn't");
		require(!star_lost(0, false), "without Shadow Stars no star is lost");

		std::cout << "SkillUse passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
