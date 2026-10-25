#pragma once

#include "PqSkills.h"

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

		// What the server last said (the context's +0x3800 and +0x38cc), kept until the character's data
		// comes again.
		struct Session
		{
			int32_t energy = 0;
			// FIELD_SET_VARIABLE `Bamboo_Used` (0xa13081), which goes with its field.
			bool bamboo_used = false;
		};

		inline Session &session()
		{
			static Session state;

			return state;
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
