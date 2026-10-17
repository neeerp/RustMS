#pragma once

#include "../../Character/Party.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// Where Mystic Doors may open, which stand in a town, and when the player
	// is in one, as the v83 client decides.
	namespace door_rules
	{
		constexpr int32_t MYSTIC_DOOR = 2311002;

		// How far from a portal a door may not open (FUN_00712ab1(pos, 0x32)).
		constexpr int16_t PORTAL_REACH = 50;

		// fieldLimit's bit against Mystic Door.
		constexpr int32_t FIELD_LIMIT = 0x8;

		// Whether `point` is in the box from `left`/`top` up to, but not
		// including, `right`/`bottom`, as PtInRect decides.
		inline bool in_box(Point<int16_t> point, int16_t left, int16_t top, int16_t right, int16_t bottom)
		{
			return point.x() >= left && point.x() < right && point.y() >= top && point.y() < bottom;
		}

		// Whether the player stands in a door: 20 px either side of it, 50 px
		// above and below (FUN_007bd594, FUN_005375ed).
		inline bool reaches(Point<int16_t> door, Point<int16_t> player)
		{
			return in_box(player, door.x() - 20, door.y() - 50, door.x() + 20, door.y() + 50);
		}

		// Whether a portal of type `type` at `portal` keeps a door from
		// opening at `player`: any but a spawn point (type 0) whose box holds
		// them.
		inline bool blocks(int32_t type, Point<int16_t> portal, Point<int16_t> player)
		{
			return type != 0 && in_box(player, portal.x() - PORTAL_REACH, portal.y() - PORTAL_REACH,
									   portal.x() + PORTAL_REACH, portal.y() + PORTAL_REACH);
		}

		enum class Cast
		{
			OPEN,
			// "You cannot use that skill on this map."
			FORBIDDEN,
			// Refused without a word.
			SILENT,
			// "You cannot use the Mystic Door Skill near the portal."
			NEAR_PORTAL
		};

		// Whether the player may open a door where they stand (FUN_00969bf9):
		// not where the map forbids it, nor off the ground, in a town or in
		// an event map (1x9xxxxxx), nor by a portal.
		inline Cast may_cast(int32_t fieldlimit, int32_t mapid, bool town, bool onground, bool near_portal)
		{
			if (fieldlimit & FIELD_LIMIT)
				return Cast::FORBIDDEN;

			if (!onground || town || (mapid / 1000000) % 100 == 9)
				return Cast::SILENT;

			if (near_portal)
				return Cast::NEAR_PORTAL;

			return Cast::OPEN;
		}

		// A door standing in town: whose, and at which of the town's door
		// portals.
		struct TownDoor
		{
			int32_t owner;
			size_t portal;

			bool operator==(const TownDoor &other) const
			{
				return owner == other.owner && portal == other.portal;
			}
		};

		// The doors standing in town `mapid` (FUN_005365c8). Alone, the
		// player's own stands at the first door portal. In a party, each
		// member's stands at the portal of their slot, and the player's own
		// is the one in their slot.
		inline std::vector<TownDoor> town_doors(int32_t mapid, int32_t self, const MysticDoor &own,
												const Party &party)
		{
			std::vector<TownDoor> doors;

			if (!party.exists())
			{
				if (own.leads_from(mapid))
					doors.push_back({ self, 0 });

				return doors;
			}

			for (const PartyMember &member : party.get_members())
				if (member.id != 0 && member.map_id != MysticDoor::NONE && member.door.leads_from(mapid))
					doors.push_back({ member.id, member.slot });

			return doors;
		}
	}
}
