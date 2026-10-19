#pragma once

#include "../../Template/Rectangle.h"

#include <algorithm>
#include <cstdint>

namespace ms
{
	// A regular attack with a bow, crossbow, claw or gun first looks for a mob
	// right in front of the user, and whacks it instead of shooting
	// (FUN_0094e256 passing FUN_00950921 an out pointer).
	namespace point_blank
	{
		// How far in front of the user the whack reaches at most.
		constexpr int16_t MAX_REACH = 65;

		// The whack's range, from the melee stance's afterimage range (lt.x is
		// the front reach, as when facing left).
		inline Rectangle<int16_t> range(Rectangle<int16_t> afterimage)
		{
			int16_t left = std::max<int16_t>(afterimage.left(), -MAX_REACH);
			left = std::min<int16_t>(left, afterimage.right() - 1);

			return { left, afterimage.right(), afterimage.top(), afterimage.bottom() };
		}

		// The shots that never whack, however close the mob. Any other skill
		// that shoots whacks a mob in front of the user with a regular attack
		// instead (FUN_007668b7, FUN_009696a2 at 0x9698ca).
		inline bool always_shoots(int32_t skill_id)
		{
			switch (skill_id)
			{
				case 3100001:
				case 3110001:
				case 3111004:
				case 3200001:
				case 3210001:
				case 3211004:
				case 4121003:
				case 4221003:
				case 5121002:
				case 5201006:
				case 5211004:
				case 5211005:
				case 5221007:
				case 5221008:
				case 11101004:
				case 13101002:
				case 13101005:
				case 13111000:
				case 14101006:
				case 15111006:
				case 15111007:
				case 21100004:
				case 21110004:
				case 21120006:
					return true;
				default:
					return false;
			}
		}

		// The Mortal Blow a whack may shoot instead: the Ranger's from 311 on,
		// the Sniper's from 321 on, none for other jobs (FUN_007656aa).
		inline int32_t mortal_blow(int32_t job)
		{
			if (job / 10 == 31 && job % 10 >= 1)
				return 3110001;

			if (job / 10 == 32 && job % 10 >= 1)
				return 3210001;

			return 0;
		}

		// A Mortal Blow says so in the flags byte of its shot (0x955228).
		constexpr uint8_t MORTAL_BLOW_FLAG = 0x04;

		// A whack after a Mortal Blow that didn't come up says so in its own
		// (0x9516e9, 0x952949).
		constexpr uint8_t MISSED_BLOW_FLAG = 0x10;
	}
}
