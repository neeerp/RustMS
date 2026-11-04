#include "../Gameplay/Combat/BoomerangStep.h"
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
		using namespace ms;
		using boomerang_step::resting;

		require(!resting(0, 0), "never used");
		require(!resting(5000, 0), "never used, later");

		// Used at 1000: gone until 1900, refused until 2400.
		require(resting(1000, 1900), "while gone");
		require(resting(1900, 1900), "just back");
		require(resting(2400, 1900), "at the end of its rest");
		require(!resting(2401, 1900), "rested");

		require(pq_skills::keeps_harm_off(1009), "Bamboo Rain");
		require(pq_skills::keeps_harm_off(10001009), "Noblesse's");
		require(pq_skills::keeps_harm_off(20001009), "Aran's");
		require(pq_skills::keeps_harm_off(20011009), "Evan's");
		require(pq_skills::keeps_harm_off(1020), "Rage of Pharaoh");
		require(pq_skills::keeps_harm_off(20011020), "Evan's Rage");
		require(!pq_skills::keeps_harm_off(1010), "Invincibility is a buff");
		require(!pq_skills::keeps_harm_off(1011), "Power Explosion");
		require(!pq_skills::keeps_harm_off(4221007), "not a secret skill");

		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}
}
