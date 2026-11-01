#pragma once

#include <cstdint>

namespace ms
{
	// Flash Jump, which throws the user on through the air, as in the
	// MapleRoyals client (FUN_0096be38), and the Dawn Warrior's Soul Rush,
	// which throws them off the ground the same way.
	namespace flash_jump
	{
		constexpr int32_t SOUL_RUSH = 11101005;

		// Soul Rush may go again 1.5 s after it went (0x9535e3).
		constexpr int64_t RUSH_DELAY = 1500;

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

		// Soul Rush pushes as Flash Jump at twice its level (0x96bfe5).
		inline int32_t push_level(int32_t skill, int32_t level)
		{
			return skill == SOUL_RUSH ? level * 2 : level;
		}

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

		// Soul Rush goes only from a foothold, off any rope or ladder, once
		// ready (0x96bf14-0x96bf4b).
		inline bool may_rush(bool onground, bool climbing, bool used, int64_t now, int64_t ready_at)
		{
			return onground && !climbing && !used && now >= ready_at;
		}

		// The effect left where the user jumped (0x96c0d4, 0x96c0a4).
		inline const char *effect(int32_t skill)
		{
			if (skill == SOUL_RUSH)
				return "SoulRush";

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
