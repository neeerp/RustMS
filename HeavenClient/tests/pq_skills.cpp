#include "../Gameplay/Combat/PqSkills.h"

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
		using namespace ms::pq_skills;

		require(kind(1009) == Kind::BAMBOO_RAIN && kind(10001009) == Kind::BAMBOO_RAIN, "Bamboo Rain");
		require(kind(1010) == Kind::DOJO_BUFF && kind(20001011) == Kind::DOJO_BUFF && kind(20011010) == Kind::DOJO_BUFF,
				"the Dojo buffs");
		require(kind(1020) == Kind::RAGE_OF_PHARAOH && kind(20001020) == Kind::RAGE_OF_PHARAOH, "Rage of Pharaoh");
		require(kind(1001) == Kind::NONE && kind(1101010) == Kind::NONE && kind(21001009) == Kind::NONE,
				"no other skill, and no job's own");

		require(allowed(Kind::DOJO_BUFF, FIELD_DOJO) && allowed(Kind::BAMBOO_RAIN, FIELD_DOJO_2), "the Dojo");
		require(!allowed(Kind::DOJO_BUFF, 0) && !allowed(Kind::DOJO_BUFF, FIELD_PYRAMID), "only the Dojo");
		require(allowed(Kind::RAGE_OF_PHARAOH, FIELD_PYRAMID) && !allowed(Kind::RAGE_OF_PHARAOH, FIELD_DOJO),
				"only the Pyramid");
		require(allowed(Kind::NONE, 0), "any other skill anywhere");

		require(refused(Kind::BAMBOO_RAIN, true, false) && refused(Kind::RAGE_OF_PHARAOH, false, true),
				"no attack riding or transformed");
		require(!refused(Kind::DOJO_BUFF, true, true) && !refused(Kind::BAMBOO_RAIN, false, false),
				"the buffs go either way");

		require(strike(Kind::BAMBOO_RAIN, 1000, false) == 1000 && strike(Kind::BAMBOO_RAIN, 1000, true) == 300,
				"Bamboo Rain takes all a mob's HP, 30% of a boss's");
		require(strike(Kind::RAGE_OF_PHARAOH, 5000, true) == 5000, "Rage of Pharaoh takes even a boss's");
		require(strike(Kind::NONE, 1000, false) == -1, "nothing for any other skill");

		require(family(0, INVINCIBILITY) == 1010 && family(312, INVINCIBILITY) == 1010, "explorers");
		require(family(1100, INVINCIBILITY) == 10001010 && family(2100, INVINCIBILITY) == 20001010, "Noblesse, Aran");
		require(family(2001, INVINCIBILITY) == 20011010 && family(2210, INVINCIBILITY) == 20011010, "Evan");

		std::cout << "pq skills tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
