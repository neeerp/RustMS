#include "../Character/Party.h"
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

		auto member = [](int32_t id, const char* name, int32_t channel, int32_t map_id)
		{
			PartyMember m;
			m.id = id;
			m.name = name;
			m.channel = channel;
			m.map_id = map_id;
			return m;
		};

		Party party;
		require(!party.exists(), "a new player has no party");
		require(!party.is_leader(0), "nobody leads a missing party");

		party.update(7, 1, { member(1, "Alice", 0, 50000), member(2, "Bobby", 0, 50000) });
		require(party.exists() && party.get_id() == 7, "an update makes the party");
		require(party.is_leader(1) && !party.is_leader(2), "the update names the leader");
		require(party.find("Bobby") && party.find("Bobby")->id == 2, "members are found by name");

		party.set_hp(2, 30, 50);
		party.update(7, 1, { member(1, "Alice", 0, 50000), member(2, "Bobby", 0, 50000), member(3, "Carol", 1, 0) });
		require(party.find(2)->hp == 30 && party.find(2)->maxhp == 50, "a member update keeps known HP");
		require(party.find(3)->hp == 0, "a new member starts without HP");

		auto nearby = party.on_map(1, 50000);
		require(nearby.size() == 1 && nearby[0]->id == 2, "only other members on the map are nearby");

		party.update(7, 1, { member(1, "Alice", 0, 50000), member(2, "Bobby", -2, 0) });
		require(!party.find(2)->online(), "channel -2 is offline");
		require(party.on_map(1, 50000).empty(), "offline members are not nearby");

		party.set_leader(2);
		require(party.is_leader(2), "the lead passes");

		party.clear();
		require(!party.exists() && party.get_members().empty(), "leaving forgets the party");

		std::cout << "Party passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
