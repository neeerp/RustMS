#include "../Gameplay/MapleMap/FriendlyRules.h"
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
		using namespace ms::friendly;

		// Only a friendly is hurt by mobs: by any with no list, else by those listed.
		require(hurt_by(true, {}, 100100), "any mob hurts a friendly with no list");
		require(!hurt_by(false, {}, 100100), "a mob that isn't friendly");
		require(hurt_by(true, { 9300062, 9300063 }, 9300063), "a listed mob");
		require(!hurt_by(true, { 9300062, 9300063 }, 100100), "a mob not listed");

		// The party's leader reports, or a player with no one else around.
		require(reports(true, 2), "the leader");
		require(reports(false, 0), "alone on the map");
		require(reports(true, 0), "a leader alone");
		require(!reports(false, 1), "a member with others around");

		// At most one report each 1.5 s, the first 1.5 s after the friendly came.
		require(!may_report(1499, 0), "too soon after it came");
		require(may_report(1500, 0), "1.5 s after it came");
		require(!may_report(3000, 1600), "too soon after the last");
		require(may_report(3100, 1600), "1.5 s after the last");

		// Bodies meet only if they share more than an edge.
		require(meet(0, 0, 10, 10, 5, 5, 15, 15), "overlapping");
		require(meet(0, 0, 10, 10, 2, 2, 4, 4), "one inside the other");
		require(!meet(0, 0, 10, 10, 10, 0, 20, 10), "side by side");
		require(!meet(0, 0, 10, 10, 0, 10, 10, 20), "one on the other");
		require(!meet(0, 0, 10, 10, 20, 20, 30, 30), "apart");
		require(!meet(0, 0, 10, 10, 5, 5, 5, 5), "an empty body inside");

		// The gauge's fill, rounded down, within 0 and 100.
		require(percent(5000, 5000) == 100, "full");
		require(percent(4937, 5000) == 98, "rounded down");
		require(percent(0, 5000) == 0, "gone");
		require(percent(-5, 5000) == 0, "below 0");
		require(percent(6000, 5000) == 100, "above full");
		require(percent(10, 0) == 0, "no max HP");
		require(percent(2000000000, 2000000000) == 100, "no overflow");

		// Two 30 ms steps grey, two not.
		require(grey(0) && grey(59), "grey first");
		require(!grey(60) && !grey(119), "then not");
		require(grey(120), "then grey again");

		std::cout << "friendly_rules: ok\n";
		return 0;
	}
	catch (const std::exception &e)
	{
		std::cerr << "friendly_rules: " << e.what() << '\n';
		return 1;
	}
}
