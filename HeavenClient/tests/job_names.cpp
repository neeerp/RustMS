#include "../Character/Job.h"
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

		// v83's names, as it spells and spaces them.
		require(Job::get_name(0) == "Beginner", "beginners");
		require(Job::get_name(100) == "Swordman", "Swordman, not Swordsman");
		require(Job::get_name(210) == "Wizard (Fire,Poison)", "the fire wizard");
		require(Job::get_name(211) == "Mage(Fire, Poison)", "the fire mage");
		require(Job::get_name(222) == "Arch Mage(Ice,Lightning)", "the ice arch mage");
		require(Job::get_name(320) == "Crossbow man", "Crossbow man");
		require(Job::get_name(412) == "Night Lord", "Night Lord");
		require(Job::get_name(920) == "MWLB", "MWLB");

		// The Cygnus Knights' tiers share the first's name; Aran's first job is Legend.
		require(Job::get_name(1100) == "Dawn Warrior" && Job::get_name(1112) == "Dawn Warrior", "Dawn Warrior");
		require(Job::get_name(1511) == "Thunder Breaker", "Thunder Breaker");
		require(Job::get_name(2000) == "Legend" && Job::get_name(2112) == "Aran", "Legend, then Aran");
		require(Job::get_name(2218) == "Evan10", "the last Evan");

		// Anything else has no name.
		require(Job::get_name(800).empty() && Job::get_name(2002).empty() && Job::get_name(113).empty(), "no name");

		require(Job(412).get_name() == "Night Lord", "a job keeps its name");

		std::cout << "Job names passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
