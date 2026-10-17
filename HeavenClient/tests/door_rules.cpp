#include "../Gameplay/MapleMap/DoorRules.h"
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
		using door_rules::Cast;

		constexpr int32_t ELLINIA = 101000000;
		constexpr int32_t FIELD = 101010000;

		// The boxes take their left and top edges, not their right and bottom.
		Point<int16_t> door(100, 200);
		require(door_rules::reaches(door, { 80, 150 }), "the door's left and top edges reach it");
		require(!door_rules::reaches(door, { 120, 200 }), "its right edge doesn't");
		require(!door_rules::reaches(door, { 100, 250 }), "nor its bottom edge");
		require(door_rules::reaches(door, { 119, 249 }), "just inside them does");

		Point<int16_t> portal(-1642, 1561);
		require(door_rules::blocks(2, portal, { -1692, 1561 }), "a portal blocks from 50 px left of it");
		require(!door_rules::blocks(2, portal, { -1592, 1561 }), "but not 50 px right");
		require(!door_rules::blocks(0, portal, portal), "a spawn point blocks nothing");

		require(door_rules::may_cast(0, FIELD, false, true, false) == Cast::OPEN, "a door opens in a field");
		require(door_rules::may_cast(0x8, FIELD, false, true, false) == Cast::FORBIDDEN, "fieldLimit 0x8 forbids it");
		require(door_rules::may_cast(0x8, ELLINIA, true, false, true) == Cast::FORBIDDEN,
				"the map's word comes first");
		require(door_rules::may_cast(0, ELLINIA, true, true, false) == Cast::SILENT, "not in a town");
		require(door_rules::may_cast(0, FIELD, false, false, false) == Cast::SILENT, "not in the air");
		require(door_rules::may_cast(0, 109010000, false, true, false) == Cast::SILENT, "not in an event map");
		require(door_rules::may_cast(0, FIELD, false, true, true) == Cast::NEAR_PORTAL, "not by a portal");

		MysticDoor own;
		Party party;
		require(door_rules::town_doors(ELLINIA, 1, own, party).empty(), "no door, none in town");

		own.town = ELLINIA;
		own.field = FIELD;
		auto solo = door_rules::town_doors(ELLINIA, 1, own, party);
		require(solo.size() == 1 && solo[0] == door_rules::TownDoor{ 1, 0 }, "alone, it stands at the first portal");
		require(door_rules::town_doors(FIELD, 1, own, party).empty(), "only in its own town");

		auto member = [](int32_t id, size_t slot, int32_t map_id, MysticDoor door)
		{
			PartyMember m;
			m.id = id;
			m.slot = slot;
			m.map_id = map_id;
			m.door = door;
			return m;
		};

		MysticDoor none;
		party.update(7, 1, { member(1, 0, ELLINIA, none), member(2, 2, FIELD, own),
							 member(3, 3, MysticDoor::NONE, own) });
		auto doors = door_rules::town_doors(ELLINIA, 1, own, party);
		require(doors.size() == 1 && doors[0] == door_rules::TownDoor{ 2, 2 },
				"in a party, each door stands at its owner's slot, the player's own too");

		MysticDoor closed = own;
		closed.field = MysticDoor::NONE;
		party.set_door(2, closed);
		require(door_rules::town_doors(ELLINIA, 1, own, party).empty(), "a closed door leads nowhere");

		std::cout << "Door rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
