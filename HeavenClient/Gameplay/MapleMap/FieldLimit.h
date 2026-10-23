#pragma once

#include <algorithm>
#include <cstdint>
#include <map>
#include <vector>

namespace ms
{
	// What a map's fieldLimit stops the v83 client doing.
	namespace field_limit
	{
		constexpr int32_t JUMP = 0x1;
		constexpr int32_t MOVEMENT_SKILLS = 0x2;
		constexpr int32_t SUMMON_SACKS = 0x4;
		constexpr int32_t RETURN_SCROLLS = 0x20;
		constexpr int32_t TELEPORT_ROCKS = 0x40;
		constexpr int32_t MAP_SCROLLS = 0x100;
		constexpr int32_t RIDING = 0x200;
		constexpr int32_t CONSUMABLES = 0x400;
		constexpr int32_t PARTY_LEADER = 0x800;
		// A guest's wedding invitation can't be opened (0xa103f4).
		constexpr int32_t INVITATIONS = 0x2000;
		constexpr int32_t WEATHER_ITEMS = 0x4000;
		constexpr int32_t DOWN_JUMP = 0x20000;
		// Miu Miu's merchant (5450000) doesn't come (0xa0ecab).
		constexpr int32_t MIU_MIU = 0x40000;
		constexpr int32_t DROPS = 0x400000;

		// A return scroll's moveTo for "the nearest town": the map's return map.
		constexpr int32_t NEAREST_TOWN = 999999999;

		// The Master Guardian, whom Dark Sight never hides the player from.
		constexpr int32_t MASTER_GUARDIAN = 9400594;

		// The kinds v83 sends as USE_ITEM (FUN_004efd25 and the quick slots
		// at FUN_0094f2ed): potions, food, cures, transformations and the
		// like. Return scrolls, scrolls and the rest go other ways.
		inline bool is_use_item(int32_t item_id)
		{
			switch (item_id / 10000)
			{
				case 200:
				case 201:
				case 202:
				case 205:
				case 221:
				case 236:
				case 238:
				case 245:
					return true;
				default:
					return false;
			}
		}

		// v83 FUN_00a092fb: with bit 0x400, only the items the map lists
		// under info/allowedItem can be used. The check covers only USE_ITEM.
		inline bool may_use(int32_t fieldlimit, const std::vector<int32_t> &allowed, int32_t item_id)
		{
			return !is_use_item(item_id) || !(fieldlimit & CONSUMABLES)
					|| std::find(allowed.begin(), allowed.end(), item_id) != allowed.end();
		}

		// v83 FUN_00a0a63f, type 0x10: weather items (512xxxx) can't be used
		// with bit 0x4000, or on fields of type 60.
		inline bool may_blow_weather(int32_t fieldlimit, int32_t fieldtype)
		{
			return !(fieldlimit & WEATHER_ITEMS) && fieldtype != 60;
		}

		// v83 FUN_00a0f9e2: chairs can't be sat on in fields of type 61,
		// the wedding hall 680000300.
		inline bool may_sit(int32_t fieldtype)
		{
			return fieldtype != 61;
		}

		// v83 FUN_004a95bf: whether the death dialog offers a Wheel of
		// Destiny here. It doesn't on fields of type 1, 3 to 5, 7, 10, 11
		// or 15, in the event maps (9xxxxxxxx), on the ships (200090xxx)
		// or in 390xxxxxx.
		inline bool may_use_wheel(int32_t fieldtype, int32_t map_id)
		{
			switch (fieldtype)
			{
				case 1:
				case 3:
				case 4:
				case 5:
				case 7:
				case 10:
				case 11:
				case 15:
					return false;
				default:
					return map_id / 100000000 != 9 && map_id / 1000 != 200090 && map_id / 1000000 != 390;
			}
		}

		// v83 FUN_00a26227: whether two maps are in the same area, by the
		// first two digits of their ids in Map/AreaCode.img. Event maps
		// (the millions digit 9) and the ships (20009xxxx) are in none.
		inline bool same_area(int32_t from, int32_t to, const std::map<int32_t, int32_t> &codes)
		{
			auto area = [&codes](int32_t map_id)
			{
				if ((map_id / 1000000) % 100 == 9 || map_id / 10000 == 20009)
					return codes.end();

				return codes.find(map_id / 10000000);
			};

			auto area_from = area(from);
			auto area_to = area(to);

			return area_from != codes.end() && area_to != codes.end() && area_from->second == area_to->second;
		}

		enum class ScrollRefusal
		{
			NONE,
			// "You can't use it here in this map."
			HERE,
			// "You cannot go to that place."
			THERE
		};

		// v83 FUN_00a1e1ce: bit 0x20 stops every return scroll, bit 0x100
		// those that name a map. A scroll that names a map stays in the
		// player's area unless it ignores continents.
		inline ScrollRefusal return_scroll_refusal(int32_t fieldlimit, int32_t map_id, int32_t move_to,
				bool ignore_continent, const std::map<int32_t, int32_t> &codes)
		{
			if (fieldlimit & RETURN_SCROLLS)
				return ScrollRefusal::HERE;

			if (move_to == NEAREST_TOWN)
				return ScrollRefusal::NONE;

			if (fieldlimit & MAP_SCROLLS)
				return ScrollRefusal::HERE;

			if (!ignore_continent && !same_area(map_id, move_to, codes))
				return ScrollRefusal::THERE;

			return ScrollRefusal::NONE;
		}

		// v83 FUN_00a0977b: bit 0x4 keeps summoning sacks shut unless the
		// map's info/allowedItem lists them.
		inline bool may_summon(int32_t fieldlimit, const std::vector<int32_t> &allowed, int32_t item_id)
		{
			return !(fieldlimit & SUMMON_SACKS) || std::find(allowed.begin(), allowed.end(), item_id) != allowed.end();
		}

		// v83 FUN_00a0977b: a summoning sack asks before it opens, unless
		// it is an event's (2109xxx) or 2100067.
		inline bool sack_asks(int32_t item_id)
		{
			return item_id / 1000 != 2109 && item_id != 2100067;
		}

		// v83 FUN_00a0a63f, type 0x25: bit 0x40000 keeps Miu Miu's merchant
		// away.
		inline bool may_call_merchant(int32_t fieldlimit)
		{
			return !(fieldlimit & MIU_MIU);
		}

		// v83 FUN_00a0900a and FUN_00a23de5: bit 0x400000 keeps items and
		// mesos off the ground.
		inline bool may_drop(int32_t fieldlimit)
		{
			return !(fieldlimit & DROPS);
		}

		// v83 FUN_0094c383 and FUN_0094c4f8: bit 0x1 stops jumping, and bit
		// 0x20000 the down-jump (Down + Jump), for all but GMs. Down + Jump
		// never falls back to a plain jump.
		inline bool may_jump(int32_t fieldlimit, bool down)
		{
			return !(fieldlimit & (down ? DOWN_JUMP : JUMP));
		}

		// v83 FUN_00678c77: Dark Sight keeps a mob's touch off the player,
		// but not where bit 0x2 is set, and never the Master Guardian's.
		inline bool may_touch(int32_t fieldlimit, bool hidden, int32_t mob_id)
		{
			return !hidden || (fieldlimit & MOVEMENT_SKILLS) || mob_id == MASTER_GUARDIAN;
		}

		// v83 FUN_00950921 and FUN_009537d5: bit 0x2 quietly stops the
		// attacks that carry the player along or push them back.
		inline bool may_attack(int32_t fieldlimit, int32_t skill_id)
		{
			if (!(fieldlimit & MOVEMENT_SKILLS))
				return true;

			switch (skill_id)
			{
				case 1121006:  // Rush
				case 1221007:
				case 1321003:
				case 4121008:  // Ninja Storm
				case 4211002:  // Assaulter
				case 4221001:  // Assassinate
				case 5101002:  // Backspin Blow
				case 5101004:  // Corkscrew Blow
				case 5121005:  // Snatch
				case 15101003: // Corkscrew Blow
				case 21100002: // Final Charge
				case 21110003: // Final Toss
				case 21110006: // Rolling Spin
				case 3121003:  // Dragon's Breath
				case 3221003:
				case 5201006:  // Recoil Shot
					return false;
				default:
					return true;
			}
		}

		// v83 FUN_00969e21: bit 0x2 stops the dashes and the transformations
		// that move the player faster, with "You cannot use that skill on
		// this map."
		inline bool may_buff(int32_t fieldlimit, int32_t skill_id)
		{
			if (!(fieldlimit & MOVEMENT_SKILLS))
				return true;

			switch (skill_id)
			{
				case 1014:     // Space Dash
				case 10001015:
				case 5001005:  // Dash
				case 15001003:
				case 5111005:  // Transformation
				case 5121003:  // Super Transformation
				case 15111002:
				case 13111005: // Albatross
					return false;
				default:
					return true;
			}
		}

		// v83 FUN_00a0a4aa: no teleport rock in the event maps (the
		// millions digit 9), or where bit 0x40 or 0x2 is set.
		inline bool may_teleport_rock(int32_t fieldlimit, int32_t map_id)
		{
			return (map_id / 1000000) % 100 != 9 && !(fieldlimit & (TELEPORT_ROCKS | MOVEMENT_SKILLS));
		}

		// v83 FUN_0090bddb and FUN_00530226: bit 0x800 stops a party leader
		// passing the role on.
		inline bool may_change_leader(int32_t fieldlimit)
		{
			return !(fieldlimit & PARTY_LEADER);
		}

		// v83 FUN_00858603: the minimap folds to its title bar in a map
		// without a miniMap, where hideMinimap is set, and in the
		// miniMapOnOff maps once the server has switched them off
		// (MINIMAP_ON_OFF 0x56). GMs see past both flags.
		inline bool shows_minimap(bool gm, bool has_minimap, bool hide, bool on_off, bool switched_on)
		{
			if (!has_minimap)
				return false;

			return gm || (!hide && (switched_on || !on_off));
		}
	}
}
