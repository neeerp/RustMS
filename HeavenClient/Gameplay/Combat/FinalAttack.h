#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace ms
{
	// Final Attack: a swing or shot that follows some skill attacks on its own, when its chance comes up
	// (0x957830). The triggering skill lists, under `finalAttack`, each Final Attack it may set off and the
	// weapon types it does so with.
	namespace final_attack
	{
		// How long after its trigger, in ms, a Final Attack follows: a third of the way from the trigger's
		// hit to the end of its action, both in ms.
		inline int32_t delay(int32_t hit, int32_t duration)
		{
			return hit + (duration - hit) / 3;
		}

		// A weapon's type, as `finalAttack` lists them.
		inline int32_t weapon_type(int32_t weapon)
		{
			return weapon / 10000 % 100;
		}

		// Whether a Final Attack with this weapon type is a swing, or a shot; with any other it does nothing
		// (0x96cd3e).
		inline bool swings(int32_t type)
		{
			return (type >= 30 && type <= 33) || (type >= 40 && type <= 44);
		}

		inline bool shoots(int32_t type)
		{
			return (type >= 45 && type <= 47) || type == 49;
		}

		// How far in front of the player a Final Attack's shot starts.
		constexpr int16_t SHOT_REACH = 65;

		// The Cygnus Knights' Final Attacks, which come up only while their buff is on.
		inline bool needs_buff(int32_t skill)
		{
			return skill == 11101002 || skill == 13101002;
		}

		// Slash Blast, an explorer's or a Dawn Warrior's.
		inline bool slash_blast(int32_t skill)
		{
			return skill == 1001005 || skill == 11001003;
		}

		// The Slash Blast a job has.
		inline int32_t slash_blast_of(int32_t job)
		{
			return job / 1000 == 1 ? 11001003 : 1001005;
		}

		// The swings whose reach stretches to their `range` once a mob is in it: Slash Blast, Coma and
		// Charged Blow (0x951433). A Final Attack after Slash Blast stretches to Slash Blast's.
		inline bool stretches(int32_t skill)
		{
			switch (skill)
			{
				case 1001005:
				case 11001003:
				case 1111005:
				case 1111006:
				case 1211002:
				case 11111003:
					return true;
				default:
					return false;
			}
		}

		// What a Final Attack after Slash Blast does to the damage of each mob it hits, in the order it hits
		// them (0x75bf50, 0xbdb380): about two thirds of the first's, two ninths of the second's, and so on.
		inline double falloff(size_t target)
		{
			static constexpr std::array<double, 10> factors = {
					0.666667, 0.222222, 0.074074, 0.024691, 0.00823, 0.002743, 0.000914, 0.000305, 0.000102, 0.000033
			};

			return target < factors.size() ? factors[target] : 0.0;
		}

		// A hit's damage after the falloff, cut down to a whole number.
		inline int32_t fall_off(int32_t damage, size_t target)
		{
			return static_cast<int32_t>(damage * falloff(target));
		}
	}
}
