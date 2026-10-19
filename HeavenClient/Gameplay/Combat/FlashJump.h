#pragma once

#include <cstdint>

namespace ms
{
	// Flash Jump, which throws the user on through the air, as in the
	// MapleRoyals client (FUN_0096be38).
	namespace flash_jump
	{
		inline bool jumps(int32_t skill)
		{
			switch (skill)
			{
				case 4111006: // Flash Jump
				case 14101004: // Flash Jump (Night Walker)
					return true;
				default:
					return false;
			}
		}

		// The push, in pixels per second: 350 forward and 250 up, with 40
		// more forward and 20 more up for every four levels (0x96c008).
		struct Impulse
		{
			double vx;
			double vy;
		};

		inline Impulse impulse(int32_t level, bool left)
		{
			int32_t steps = level / 4;
			double vx = 350.0 + 40.0 * steps;

			return { left ? -vx : vx, -250.0 - 20.0 * steps };
		}

		// It goes only in the air and off any rope or ladder, once until the
		// user lands or grabs one, and, out of water, not once the user is
		// falling (0x96bf52-0x96bf91, 0x96c1aa, 0x96c1d3; reset at 0x9506f6).
		inline bool may_jump(bool onground, bool climbing, bool used, bool swimming, double vspeed)
		{
			return !onground && !climbing && !used && (swimming || vspeed <= 0.0);
		}

		// The effect left where the user jumped (0x96c0d4).
		inline const char *effect(int32_t skill)
		{
			return skill == 14101004 ? "Flying1" : "Flying";
		}

		// The effect another player's jump leaves, by their job, when their
		// path shows it (0x97feb8). Soul Rush is the Dawn Warrior's.
		inline const char *remote_effect(int16_t job)
		{
			if (job >= 410 && job <= 412)
				return "Flying";

			if (job == 1400 || (job >= 1410 && job <= 1412))
				return "Flying1";

			if (job == 1100 || (job >= 1110 && job <= 1112))
				return "SoulRush";

			return nullptr;
		}
	}
}
