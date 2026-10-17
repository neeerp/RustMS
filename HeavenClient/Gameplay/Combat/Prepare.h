#pragma once

#include <algorithm>
#include <cstdint>
#include <string>

namespace ms
{
	// The skills a user prepares by holding their key, as in the MapleRoyals
	// client.
	namespace prepare
	{
		constexpr int32_t HURRICANE = 3121004;
		constexpr int32_t RAPID_FIRE = 5221004;
		constexpr int32_t WIND_ARCHER_HURRICANE = 13111002;
		constexpr int32_t PIERCING_ARROW = 3221001;
		constexpr int32_t CORKSCREW_BLOW = 5101004;
		constexpr int32_t CORKSCREW_BLOW_TB = 15101003;
		constexpr int32_t EXPLOSION = 2111002;
		constexpr int32_t CHAKRA = 4211001;

		// Whether the skill is held down to charge or keep firing
		// (FUN_004fb08f).
		inline bool keydown(int32_t skill)
		{
			switch (skill)
			{
			case 1121001: // Monster Magnet (Hero)
			case 1221001: // Monster Magnet (Paladin)
			case 1321001: // Monster Magnet (Dark Knight)
			case 2121001: // Big Bang (Fire/Poison)
			case 2221001: // Big Bang (Ice/Lightning)
			case 2321001: // Big Bang (Bishop)
			case HURRICANE:
			case PIERCING_ARROW:
			case CORKSCREW_BLOW:
			case 5201002: // Grenade
			case RAPID_FIRE:
			case WIND_ARCHER_HURRICANE:
			case 14111006: // Poison Bomb
			case CORKSCREW_BLOW_TB:
			case 22121000: // Evan
			case 22151001: // Evan
				return true;
			default:
				return false;
			}
		}

		// Whether a held skill keeps firing until it is let go, rather than
		// charging up to one attack.
		inline bool repeats(int32_t skill)
		{
			return skill == HURRICANE || skill == RAPID_FIRE || skill == WIND_ARCHER_HURRICANE;
		}

		// How long a held skill charges, in ms (FUN_00500971). It is 0 for
		// the rest.
		inline int32_t charge_cap(int32_t skill)
		{
			if (!keydown(skill))
				return 0;

			return repeats(skill) ? 2000 : 1000;
		}

		// Whether the attack packet carries how long the skill was charged.
		// Close attacks and shots carry it for every held skill, even one
		// that keeps firing; spells only for Big Bang and Evan's two.
		inline bool sends_charge(int32_t skill, bool spell)
		{
			if (!spell)
				return keydown(skill);

			switch (skill)
			{
			case 2121001:
			case 2221001:
			case 2321001:
			case 22121000:
			case 22151001:
				return true;
			default:
				return false;
			}
		}

		// Whether the skill is held here: the throws let go their own way, and
		// aren't yet.
		inline bool held(int32_t skill)
		{
			switch (skill)
			{
			case 5201002:
			case 14111006:
				return false;
			default:
				return keydown(skill);
			}
		}

		// Whether the skill is timed instead: it prepares for as long as its
		// action plays, then goes off by itself (0x96b03b, 0x96d01e).
		inline bool timed(int32_t skill)
		{
			return skill == EXPLOSION || skill == CHAKRA;
		}

		// Whether Chakra may be used: only below half HP, in whole percent
		// (0x96ae6d).
		inline bool chakra_allowed(int32_t hp, int32_t maxhp)
		{
			return maxhp > 0 && hp * 100 / maxhp < 50;
		}

		// The v83 number of a prepare action, or -1 for none: these skills'
		// `dash` is no action, so the user keeps their stance (0x4a8d14).
		inline int16_t action_id(const std::string &action)
		{
			if (action == "shoot1")
				return 0x16;

			if (action == "alert3")
				return 0x2a;

			if (action == "alert5")
				return 0x2c;

			if (action == "rapidfire")
				return 0x63;

			return -1;
		}

		// The prepare action a v83 number names, or "" for no action or one
		// these skills don't use.
		inline std::string action_name(int16_t action)
		{
			for (const char *name : { "shoot1", "alert3", "alert5", "rapidfire" })
				if (action_id(name) == action)
					return name;

			return "";
		}

		// The frame a held action stays on: Rapid Fire's first, the others'
		// third (0x4547eb).
		inline uint8_t held_frame(const std::string &action)
		{
			return action == "rapidfire" ? 0 : 2;
		}

		// The prepare packet's action and facing: the action's low 15 bits,
		// and the top bit set when facing left.
		inline uint16_t action_and_facing(int16_t action, bool toleft)
		{
			return static_cast<uint16_t>((action & 0x7fff) | (toleft ? 0x8000 : 0));
		}

		// The action in a prepare packet's action and facing, and whether it
		// faces left.
		inline int16_t prepared_action(uint16_t action_and_facing)
		{
			return static_cast<int16_t>(action_and_facing & 0x7fff);
		}

		inline bool prepared_toleft(uint16_t action_and_facing)
		{
			return (action_and_facing & 0x8000) != 0;
		}

		// Whether another user's attack leaves the skill they hold as it is:
		// the shooters keep firing with it (0x98044f).
		inline bool outlasts_attacks(int32_t skill)
		{
			return repeats(skill) || skill == 14111006;
		}

		// A skill that keeps firing shoots once more than 100 ms have passed
		// since its last shot (0x94ba6a). v83's clock moves 30 ms a step, so
		// that is every 120 ms.
		constexpr int32_t SHOT_INTERVAL = 120;

		// Whether a skill that keeps firing is due to shoot again.
		inline bool shot_due(int64_t now, int64_t last)
		{
			return now - last >= SHOT_INTERVAL;
		}

		// The charge gauge: 72 px wide, filled from x = 2 through this, a
		// pixel every 15 ms of the charge (30 ms for a 2 s cap), 2 px down
		// (0x94ba06, 0x95ca8d).
		constexpr int32_t GAUGE_START = 2;
		constexpr int32_t GAUGE_END = 69;

		inline int32_t gauge(int32_t skill, int32_t held)
		{
			int32_t step = charge_cap(skill) == 2000 ? 30 : 15;

			return std::min(held / step + GAUGE_START, GAUGE_END);
		}

		// Whether a held skill shows the gauge (0x95c17b): the ones that keep
		// firing, and Evan's, don't.
		inline bool shows_gauge(int32_t skill)
		{
			return keydown(skill) && !repeats(skill) && skill != 22121000 && skill != 22151001;
		}

		// Which gauge a held skill shows: Monster Magnet's and Corkscrew
		// Blow's own (1), or the others' (0) (0x95c456).
		inline int32_t gauge_kind(int32_t skill)
		{
			switch (skill)
			{
			case 1121001:
			case 1221001:
			case 1321001:
			case CORKSCREW_BLOW:
			case CORKSCREW_BLOW_TB:
				return 1;
			default:
				return 0;
			}
		}

		// The charge a held skill is let go with: how long it was held, from
		// 30 ms up to its cap (FUN_0095bedf).
		inline int32_t release_charge(int32_t skill, int32_t held)
		{
			return std::clamp(held, 30, charge_cap(skill));
		}

		// The percent of its damage a charged hit does: 10% plus 90% of how
		// full the charge is, cut down to a whole percent (0x791eb1,
		// 0x79048c). An attack sent with no charge does it all.
		inline int32_t charge_percent(int32_t skill, int32_t charge)
		{
			int32_t cap = charge_cap(skill);

			if (charge == 0 || cap == 0)
				return 100;

			return 10 + charge * 90 / cap;
		}
	}
}
