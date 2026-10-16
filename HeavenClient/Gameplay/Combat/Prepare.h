#pragma once

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
	}
}
