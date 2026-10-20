#include "../Character/CatchItemRules.h"
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
		using namespace ms::catch_item;

		require(no_mob_notice(0, "") == "Unable to find a tamable monster in this area.", "the perfume");
		require(no_mob_notice(1, "") == "Unable to capture the monster; the ghost is too far.", "the pouch");
		require(no_mob_notice(2, "") == "Unable to capture the monster; the monster is too far.", "the rock");
		require(no_mob_notice(3, "Too far.") == "Too far." && no_mob_notice(3, "").empty(), "its own");
		require(no_mob_notice(4, "").empty(), "the net says nothing");

		require(fail_notice(0, "") == "Unable to capture the monster; the monster is too strong.", "too strong");
		require(fail_notice(1, "") == "Element Rock cannot be used right after usage.", "too soon");
		require(fail_notice(1, "You cannot use the Fishing Net yet.") == "You cannot use the Fishing Net yet.",
			"the net's own wait");
		require(fail_notice(2, "").empty(), "else nothing");

		require(no_room_notice(1902000) == "You do not have enough slots in your item inventory.", "a hog");
		require(no_room_notice(2022323) == "Use item inventory is full." &&
			no_room_notice(4031830) == "Etc item inventory is full.", "a stack");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	return 0;
}
