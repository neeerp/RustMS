#include "../Character/QuestRequirements.h"

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
		using namespace ms::QuestRequirements;

		// The level line (report b004 1.2), quirks and all.
		require(levels(false, 0, 0) == "No limit", "no start check");
		require(levels(true, 0, 0).empty(), "no bounds");
		require(levels(true, 10, 0) == "Over Level 10", "a lower bound");
		require(levels(true, 0, 30) == " Under Level 30", "an upper bound leads with a space");
		require(levels(true, 10, 30) == "Over Level 10  Under Level 30", "both, two spaces apart");

		// Quests from Quest.nx (report b004 section 3).
		require(jobs({}) == "Available to all", "anyone");
		require(jobs({ 0 }) == "Available to Beginner", "1000: beginners");
		require(jobs({ 100, 110, 120, 130, 111, 121, 131, 112, 122, 132 }) == "Available to Warrior", "2400: a class");
		require(jobs({ 110, 120, 130, 111, 121, 131, 112, 122, 132 })
				== "At least at 2nd job advancement Available to Warrior", "2300: from the 2nd job");
		require(jobs({ 312, 322 }) == "4th job advancement Available to Bowman", "6100: one class's 4th job");
		require(jobs({ 112, 122, 132, 212, 222, 232, 312, 322, 412, 422, 512, 522 })
				== "4th job advancement Available to Warrior, Magician, Bowman, Thief, Pirate", "6006: every 4th job");
		require(jobs({ 400, 410, 420, 411, 421, 412, 422, 1400, 1410, 1411, 1412 })
				== "Available to Thief, Night Walker", "2044: a class and a Cygnus class");
		require(jobs({ 1000, 1100, 1110, 1111, 1112, 1200, 1210, 1211, 1212, 1300, 1310, 1311, 1312,
					   1400, 1410, 1411, 1412, 1500, 1510, 1511, 1512 }) == "Available to Cygnus Knights", "3530: Cygnus");
		require(jobs({ 2000, 2100, 2110, 2111, 2112 }) == "Available to Aran", "3539: Legend goes with the Aran");
		require(jobs({ 2110, 2111, 2112 }) == "Available to Aran, Aran, Aran", "2310: Aran's tiers, one each");
		require(jobs({ 2110 }) == "Available to Aran", "21300");

		// Partial lists name each job; a beginner always comes first.
		require(jobs({ 410, 0 }) == "Available to Beginner, Assassin", "one job");
		require(jobs({ 1000, 1100 }) == "Available to Noblesse, Dawn Warrior", "a Cygnus start");
		require(jobs({ 1100, 1110 }) == "Available to Dawn Warrior, Dawn Warrior", "Cygnus tiers repeat");
		require(jobs({ 1110, 1111 }) == "At least at 2nd job advancement Available to Dawn Warrior", "a Cygnus range");
		require(jobs({ 2001, 2200 }) == "Available to Evan0, Evan1", "v83's Evan names");

		// The advancement form fails on a partial tier, a gap, or slots over different tiers.
		require(jobs({ 110, 111 }) == "Available to Fighter, Crusader", "a branch of three");
		require(jobs({ 312, 322, 310, 320 }) == "Available to Bowmaster, Marksman, Hunter, Crossbow man", "a gap");
		require(jobs({ 312, 322, 411, 421, 412, 422 }) == "Available to Bowmaster, Marksman, Hermit, Chief Bandit, Night Lord, Shadower",
				"different tiers");
		require(jobs({ 310, 320, 311, 321 }) == "At least at 2nd job advancement Available to Bowman", "a range only starts");

		// Everyone but a set, by length alone.
		std::vector<int32_t> others;

		for (int32_t i = 0; i < 75; i++)
			others.push_back(100 + i);

		require(jobs(others) == "Not available for Beginners, Noblesses, and Legends.", "75 ids");

		std::cout << "Quest requirements passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
