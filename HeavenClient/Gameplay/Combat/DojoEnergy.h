#pragma once

#include "PqSkills.h"

#include <array>
#include <cstdint>

namespace ms
{
	// The Mu Lung Dojo's energy bar. The server sends the energy (SESSION_VALUE `energy`, 0xa12bd6) as attacks
	// and hits fill it; once it is full, Ctrl and Alt with an arrow held cast one of the secret skills, and the
	// server empties it again (FUN_00529968).
	namespace dojo_energy
	{
		constexpr int32_t FULL = 10000;

		// How tall the bar's coloured strip is (0x5545b2).
		constexpr int16_t GAGE = 77;

		// The height of the dark cover over the part of the strip not yet filled, from its top (0x554fc1).
		inline int16_t cover(int32_t energy)
		{
			int32_t clamped = energy < 0 ? 0 : (energy > FULL ? FULL : energy);

			return static_cast<int16_t>(GAGE - clamped * GAGE / FULL);
		}

		enum class Arrow
		{
			NONE,
			UP,
			LEFT,
			RIGHT
		};

		// The secret skill an arrow held casts (FUN_0094ce06): Up for Bamboo Rain, unless it was used on this
		// stage, Left for Invincibility and Right for Power Explosion, each the job's own copy. 0 for none.
		inline int32_t secret_skill(int32_t job, Arrow arrow, bool bamboo_used)
		{
			switch (arrow)
			{
				case Arrow::UP:
					return bamboo_used ? 0 : pq_skills::family(job, 1009);
				case Arrow::LEFT:
					return pq_skills::family(job, 1010);
				case Arrow::RIGHT:
					return pq_skills::family(job, 1011);
				default:
					return 0;
			}
		}

		// The HUD's clock (0x55522b, ticked by 0x555626): CLOCK kind 2 counts down from its seconds, and the
		// HUD shows whole seconds left, truncated, down to 0, where it stops. Its digits are drawn first when
		// the reading changes, so until then only the bar shows.
		class Countdown
		{
		public:
			void start(int64_t now_ms, int32_t seconds)
			{
				deadline = now_ms + seconds * 1000LL;
				running = true;
				shown = true;
				changed = false;
				reading = seconds;
			}

			// True on the tick it runs out, when the HUD plays `dojang/timeOver`.
			bool tick(int64_t now_ms)
			{
				if (!running)
					return false;

				int64_t left = (deadline - now_ms) / 1000;
				bool over = left <= 0;
				auto next = over ? 0 : static_cast<int32_t>(left);

				if (over)
					running = false;

				if (next != reading)
				{
					reading = next;
					changed = true;
				}

				return over;
			}

			// Nothing but the frame shows until a CLOCK comes.
			bool is_shown() const
			{
				return shown;
			}

			int32_t get_reading() const
			{
				return reading;
			}

			bool has_digits() const
			{
				return changed;
			}

		private:
			int64_t deadline = 0;
			bool running = false;
			bool shown = false;
			bool changed = false;
			int32_t reading = 0;
		};

		// What the server last said (the context's +0x3800 and +0x38cc), kept until the character's data
		// comes again.
		struct Session
		{
			int32_t energy = 0;
			// FIELD_SET_VARIABLE `Bamboo_Used` (0xa13081), which goes with its field.
			bool bamboo_used = false;
			// The field's clock, which the next field starts without.
			Countdown clock;
		};

		inline Session &session()
		{
			static Session state;

			return state;
		}

		// The HUD's two gauges (0x554792), 305 pixels at full, from `player/Gage`, centred on y 32.
		constexpr int16_t GAUGE_WIDTH = 305;
		constexpr int16_t GAUGE_Y = 32;
		// The player's ends at x 318 and loses HP from its left; the monster's starts at x 482.
		constexpr int16_t PLAYER_GAUGE_RIGHT = 318;
		constexpr int16_t MONSTER_GAUGE_LEFT = 482;

		// No clamp: the server keeps HP within its maximum.
		inline int32_t player_gauge(int32_t hp, int32_t maxhp)
		{
			return maxhp == 0 ? 0 : hp * GAUGE_WIDTH / maxhp;
		}

		// From the boss's last SHOW_MOB_HP percent, kept to 100.
		inline int32_t monster_gauge(int32_t percent)
		{
			return (percent > 100 ? 100 : percent) * GAUGE_WIDTH / 100;
		}

		// The clock's four digits (0x555700): the minutes' tens and ones, then the seconds', always two of
		// each, with only the last two digits of the minutes.
		inline std::array<int32_t, 4> clock_digits(int32_t seconds)
		{
			int32_t minutes = seconds / 60;
			int32_t rest = seconds % 60;

			return { minutes / 10 % 10, minutes % 10, rest / 10 % 10, rest % 10 };
		}

		// Ctrl and Alt pressed one right after the other, in either order (FUN_0094c79c). The first is held
		// back until the next key goes down; then either the other completes the chord, or it is let through
		// before that key. A key going up lets it through as well.
		class Chord
		{
		public:
			enum class Modifier
			{
				NONE,
				CTRL,
				ALT
			};

			enum class Result
			{
				// Handle the key as usual, after any key held back.
				PASS,
				// Hold the key back.
				HOLD,
				// The chord is complete: the key held back goes with it.
				CHORD
			};

			Result press(Modifier modifier)
			{
				Modifier held = pending;
				pending = Modifier::NONE;

				if (modifier == Modifier::NONE)
					return Result::PASS;

				if (held != Modifier::NONE && held != modifier)
					return Result::CHORD;

				pending = modifier;

				return Result::HOLD;
			}

			void reset()
			{
				pending = Modifier::NONE;
			}

		private:
			Modifier pending = Modifier::NONE;
		};
	}
}
