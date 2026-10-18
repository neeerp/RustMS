#pragma once

#include "Buff.h"

#include <cstdint>
#include <optional>
#include <string>

namespace ms
{
	// What the diseases a mob's skill gives the player keep them from doing
	// (v83 CUserLocal). A disease's reason is the mob skill | level << 16.
	namespace disease
	{
		constexpr int32_t DISPEL = 2311001;
		constexpr int32_t UNDEAD = 133;

		// Under Stop Motion, the ms the player may stand still between hurts
		// (0x94b1d7).
		constexpr int64_t STILL_TIME = 2000;

		// The mob skill a disease's reason names, and its level.
		constexpr int32_t skill(int32_t reason)
		{
			return reason & 0xFFFF;
		}

		constexpr int32_t level(int32_t reason)
		{
			return reason >> 16;
		}

		// The stats a mob skill's disease sets.
		constexpr Buffstat::Id STATS[] = {
			Buffstat::STUN, Buffstat::POISON, Buffstat::SEAL, Buffstat::DARKNESS, Buffstat::WEAKEN,
			Buffstat::CURSE, Buffstat::SLOW, Buffstat::SEDUCE, Buffstat::CONFUSE, Buffstat::UNDEAD,
			Buffstat::STOP_POTION, Buffstat::STOP_MOTION, Buffstat::FEAR
		};

		constexpr bool is_disease(Buffstat::Id stat)
		{
			for (Buffstat::Id disease : STATS)
				if (stat == disease)
					return true;

			return false;
		}

		// The stat a buff's bit gives for `reason`: bit 46 is Puppet's, or
		// Undead's when a mob skill gave it (CTS 46 in both).
		constexpr Buffstat::Id given(Buffstat::Id stat, int32_t reason)
		{
			if (stat == Buffstat::PUPPET && skill(reason) == UNDEAD && level(reason) > 0)
				return Buffstat::UNDEAD;

			return stat;
		}

		// Which diseases are on the player, with the n of the ones it
		// matters for.
		struct State
		{
			bool stunned = false;
			bool sealed = false;
			bool weakened = false;
			bool confused = false;
			bool stop_potion = false;
			std::optional<int32_t> seduced;
			// Stop Motion's n: the HP standing still costs.
			std::optional<int32_t> stop_motion;
		};

		// One frame's arrow input: x is -1 left or 1 right, y -1 up or 1
		// down.
		struct Input
		{
			int8_t x = 0;
			int8_t y = 0;
			bool jump = false;

			constexpr bool operator==(const Input &other) const
			{
				return x == other.x && y == other.y && jump == other.jump;
			}
		};

		// The input a seduced player is made to give (FUN_0095cd5b): 1 walks
		// left, 2 right, 3 jumps, 4 holds down, 6 and 7 jump left and right.
		constexpr Input seduced(int32_t n)
		{
			switch (n)
			{
			case 1: return { -1, 0, false };
			case 2: return { 1, 0, false };
			case 3: return { 0, 0, true };
			case 4: return { 0, 1, false };
			case 6: return { -1, 0, true };
			case 7: return { 1, 0, true };
			default: return {};
			}
		}

		// The input the player gives for the arrow keys held (FUN_009cbefb):
		// none while stunned, the seduction's while seduced, and every arrow
		// reversed while confused.
		constexpr Input steer(int8_t x, int8_t y, const State &state)
		{
			if (state.stunned)
				return {};

			if (state.seduced)
				return seduced(*state.seduced);

			if (state.confused)
				return { static_cast<int8_t>(-x), static_cast<int8_t>(-y), false };

			return { x, y, false };
		}

		// Jumping is refused while stunned, weakened or seduced (FUN_0094c383).
		constexpr bool can_jump(const State &state)
		{
			return !state.stunned && !state.weakened && !state.seduced;
		}

		// Attacking is refused while stunned or seduced.
		constexpr bool can_attack(const State &state)
		{
			return !state.stunned && !state.seduced;
		}

		// So are skills, and while sealed every skill but Dispel
		// (FUN_00966f7a).
		constexpr bool can_use_skill(int32_t skillid, const State &state)
		{
			return can_attack(state) && (!state.sealed || skillid == DISPEL);
		}

		// Items are refused, without a word, under Stop Potion or a
		// seduction (FUN_0094fa45, from the quickslots at 0x94f86c and the
		// USE_ITEM sender FUN_00a092fb).
		constexpr bool can_use_item(const State &state)
		{
			return !state.stop_potion && !state.seduced;
		}

		// What Stop Motion takes from a player with `hp` who stood still:
		// its n, but never the last HP (0x94b1d7).
		constexpr int32_t stop_motion_damage(int32_t n, int32_t hp)
		{
			return n < hp - 1 ? n : (hp > 1 ? hp - 1 : 0);
		}

		// The speed the player walks at: Slow's n in place of their own
		// (0x92ef56).
		constexpr int32_t walk_speed(int32_t speed, std::optional<int32_t> slow)
		{
			return slow ? *slow : speed;
		}

		// What the player is told when a mob skill gives them a disease
		// (FUN_00932da6, StringPool 0xb5a-0xb5f, 0xec2, 0xec3 and
		// 0x1558-0x155a). Slow and Confuse say nothing.
		inline std::string notice(int32_t mobskill)
		{
			switch (mobskill)
			{
			case 120: return "You skill has been sealed up, making you unable to use it.";
			case 121: return "You are in a state of darkness, so your accuracy will be decreased.";
			case 122: return "You've been weakened, making you unable to jump.";
			case 123: return "You've been stunned, making you unable to move.";
			case 124: return "You've been cursed, so the amount of EXP you'll receive will decrease.";
			case 125: return "You have been poisoned, so your HP will slowly decrease.";
			case 128: return "You have been seduced, which disables you from moving.";
			case 133: return "You will become an Undead and the effects of your Recovery will diminish by 50%.";
			case 134: return "Your current condition will not permit you to use a potion.";
			case 135: return "You will receive damage if you become covered with the Shadow of Darkness and cannot move.";
			case 136: return "The fear of darkness has overtaken you! Your range of vision has been narrowed.";
			default: return "";
			}
		}

		// What the player is told when Stop Motion hurts them (StringPool
		// 0x1561, chat type 7).
		constexpr const char *STOP_MOTION_HURT = "You receive damage having been covered by the Shadow of Darkness.";
	}
}
