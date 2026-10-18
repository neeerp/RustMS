#include "../Gameplay/Combat/SkillWeapon.h"
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
		using namespace ms::skill_weapon;
		namespace W = ms::Weapon;

		require(check(3001004, true, W::BOW, W::NONE) == FITS && check(3001004, true, W::CROSSBOW, W::NONE) == FITS,
				"an archer's shot takes a bow or a crossbow");
		require(check(3001004, true, W::CLAW, W::NONE) == WEAPON, "but nothing else, whatever Skill.img says");
		require(check(4001344, true, W::BOW, W::CLAW) == WEAPON && check(5201001, true, W::CLAW, W::GUN) == WEAPON,
				"a thief's needs a claw, a pirate's a gun");
		require(check(13001003, true, W::CROSSBOW, W::NONE) == FITS && check(14001004, true, W::CLAW, W::CLAW) == FITS,
				"Cygnus shots go by the job too");
		require(check(3101005, true, W::CROSSBOW, W::BOW) == EQUIPPED, "then the weapon Skill.img names");
		require(check(15111007, true, W::KNUCKLE, W::KNUCKLE) == FITS && check(15111007, true, W::BOW, W::KNUCKLE) == EQUIPPED,
				"Shark Wave skips the job's weapon");

		require(check(1121008, false, W::AXE_2H, W::NONE) == FITS && check(1121008, false, W::SPEAR, W::NONE) == WEAPON,
				"Brandish takes a sword or an axe");
		require(check(5101004, false, W::GUN, W::NONE) == WEAPON && check(5201002, false, W::KNUCKLE, W::NONE) == WEAPON
				&& check(14111006, false, W::BOW, W::CLAW) == WEAPON, "the held skills say which weapon");
		require(check(3121004, true, W::CLAW, W::BOW) == QUIET && check(5221004, true, W::GUN, W::GUN) == FITS,
				"held shooters refuse quietly");

		require(check(1311001, false, W::SWORD_2H, W::SPEAR) == EQUIPPED && check(1311001, false, W::SPEAR, W::SPEAR) == FITS,
				"the rest go by Skill.img");
		require(check(5001001, false, W::GUN, W::NONE) == FITS && check(5101002, false, W::GUN, W::NONE) == EQUIPPED,
				"with v83's own exceptions");
		require(check(2001005, false, W::NONE, W::NONE) == FITS, "a skill that names no weapon takes any");

		std::cout << "Skill weapon passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
