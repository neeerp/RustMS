#include "../Character/BuddyList.h"
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

		auto buddy = [](int32_t id, const char* name, int32_t channel, uint8_t flag = 0)
		{
			Buddy b;
			b.id = id;
			b.name = name;
			b.group = "Default Group";
			b.channel = channel;
			b.flag = flag;
			return b;
		};

		using Arrival = BuddyList::Arrival;
		constexpr int32_t OWN = 1;

		BuddyList list;
		require(list.get_buddies().empty() && !list.is_full(), "a new player has no buddies");
		require(list.get_capacity() == BuddyList::DEFAULT_CAPACITY, "the list starts at the default size");

		list.update({ buddy(2, "Bobby", 0), buddy(3, "Carol", -1) });
		require(list.find(2) && list.find(2)->online(), "a buddy on a channel is online");
		require(!list.find(3)->online(), "channel -1 is offline");
		require(list.online_ids() == std::vector<int32_t>{ 2 }, "only online buddies hear buddy chat");

		require(list.set_channel(3, OWN, false, OWN) == Arrival::LOGGED_IN, "coming online is told first");
		require(list.set_channel(2, -1, false, OWN) == Arrival::NONE, "logging out tells nothing");
		require(list.online_ids() == std::vector<int32_t>{ 3 }, "channel updates move buddies on and off");
		require(list.set_channel(3, OWN, false, OWN) == Arrival::NONE, "an unchanged channel tells nothing");
		require(list.set_channel(3, 0, false, OWN) == Arrival::NONE, "leaving the player's channel tells nothing");
		require(list.set_channel(3, OWN, false, OWN) == Arrival::SAME_CHANNEL, "coming to the player's channel is told");
		require(list.set_channel(3, OWN, true, OWN) == Arrival::NONE && list.find(3)->in_cash_shop,
			"the Cash Shop byte alone is stored but tells nothing");
		require(list.set_channel(9, 0, false, OWN) == Arrival::NONE && !list.find(9), "an update for a stranger adds nobody");

		// Requests stay in the list, unseen and uncounted.
		list.update({ buddy(2, "Bobby", 0), buddy(4, "Dave", 0, 1) });
		list.add(buddy(5, "Erin", 2, 1));
		require(list.get_buddies().size() == 3 && list.listed() == 1, "requests aren't counted");
		require(list.find(5) && list.find(5)->pending(), "a request is kept");
		require(list.online_ids() == std::vector<int32_t>{ 2 }, "requests don't hear buddy chat");
		require(list.online_ids(std::set<int32_t>{ 2 }).empty() && list.online_ids("Default Group") == std::vector<int32_t>{ 2 }
			&& list.online_ids("Default Group", std::set<int32_t>{ 2 }).empty(), "nor do blocked buddies");
		require(!list.find_listed("dave") && list.find_listed("bOBBY") == list.find(2),
			"names are found regardless of case, requests left out");

		list.replace(4, buddy(4, "Dave", 0));
		require(!list.find(4)->pending() && list.listed() == 2, "an update overwrites the entry");
		list.replace(6, buddy(6, "Fred", 0));
		require(!list.find(6), "an update for a stranger adds nobody");

		Buddy hidden = buddy(7, "Gail", 0, 2);
		require(!hidden.online() && !hidden.hears_chat(), "flag 2 is always offline");
		require(buddy(7, "Gail", -1, 5).online(), "flags past 4 are always online");

		list.set_capacity(2);
		require(list.is_full(), "the list fills at its capacity");

		list.update({});
		require(list.get_buddies().empty(), "an empty list clears it");

		std::cout << "BuddyList passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
