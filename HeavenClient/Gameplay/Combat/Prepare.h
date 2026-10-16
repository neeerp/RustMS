#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// The skills a user prepares by holding their key, as in the MapleRoyals
	// client.
	namespace prepare
	{
		constexpr int32_t HURRICANE = 3121004;
		constexpr int32_t RAPID_FIRE = 5221004;
		constexpr int32_t WIND_ARCHER_HURRICANE = 13111002;

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
			case 3221001: // Piercing Arrow
			case 5101004: // Corkscrew Blow
			case 5201002: // Grenade
			case RAPID_FIRE:
			case WIND_ARCHER_HURRICANE:
			case 14111006: // Poison Bomb
			case 15101003: // Corkscrew Blow (Thunder Breaker)
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

		// The charge gauge: 72 px wide, filled from x = 2 to this, a pixel
		// every 15 ms of the charge (30 ms for a 2 s cap) (0x94ba06).
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
