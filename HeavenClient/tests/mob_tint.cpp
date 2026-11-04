#include "../Gameplay/Combat/MobTint.h"
#include "../Gameplay/Combat/DamageLayout.h"

#include <cstdlib>
#include <iostream>

namespace
{
	void require(bool condition, const char *message)
	{
		if (!condition)
		{
			std::cerr << message << std::endl;
			std::exit(1);
		}
	}
}

int main()
{
	using namespace ms;

	require(mob_tint::of_reason(2201004) == mob_tint::ICE, "Cold Beam ices blue");
	require(mob_tint::of_reason(21120006) == mob_tint::ICE, "so does Combo Tempest");
	require(mob_tint::of_reason(21111005) == mob_tint::ICE, "and Snow Charge");
	require(mob_tint::of_reason(5211004) == mob_tint::FIRE, "Flamethrower burns red");
	require(mob_tint::of_reason(2111006) == mob_tint::POISON, "Element Composition poisons green");
	require(mob_tint::of_reason(2221003) == mob_tint::ICE_DEMON, "Ice Demon has its own");
	require(mob_tint::of_reason(2121006) == mob_tint::PARALYZE, "Paralyze greys");
	require(mob_tint::of_reason(1111005) == mob_tint::NONE, "a stun has none");
	require(mob_tint::of_reason(0x0005008F) == mob_tint::NONE, "nor a mob's skill");
	require(mob_tint::of_reason(0x00012714) == mob_tint::ICE, "the low bits name the rest");

	uint32_t tint = mob_tint::over(mob_tint::NONE, 2201004);
	require(tint == mob_tint::ICE, "the first colour takes");
	require(mob_tint::over(tint, 1111005) == mob_tint::ICE, "a reason with none leaves it");
	require(mob_tint::over(tint, 2111006) == mob_tint::POISON, "the last colour wins");

	require(!damage_layout::shows_number(21120006), "Combo Tempest shows no number");
	require(damage_layout::shows_number(21110004), "Combo Fenrir does");

	return 0;
}
