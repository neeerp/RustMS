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

		auto buddy = [](int32_t id, const char* name, int32_t channel)
		{
			Buddy b;
			b.id = id;
			b.name = name;
			b.group = "Default Group";
			b.channel = channel;
			return b;
		};

		BuddyList list;
		require(list.get_buddies().empty() && !list.is_full(), "a new player has no buddies");
		require(list.get_capacity() == BuddyList::DEFAULT_CAPACITY, "the list starts at the default size");

		list.update({ buddy(2, "Bobby", 0), buddy(3, "Carol", -1) });
		require(list.find(2) && list.find(2)->online(), "a buddy on a channel is online");
		require(!list.find(3)->online(), "channel -1 is offline");
		require(list.online_ids() == std::vector<int32_t>{ 2 }, "only online buddies hear buddy chat");

		list.set_channel(3, 2);
		list.set_channel(2, -1);
		require(list.online_ids() == std::vector<int32_t>{ 3 }, "channel updates move buddies on and off");
		list.set_channel(9, 0);
		require(!list.find(9), "an update for a stranger adds nobody");

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
