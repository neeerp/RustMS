#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// The pace of the guild contract a party member is asked to sign (v83
	// report b337, Update 0x592f34). The scroll unrolls (500 ms, then eight
	// frames of 120), a second later the contract's text fades in over one,
	// two seconds after that the buttons show, and thirty seconds after the
	// text began the client refuses for the player.
	struct GuildMakeTimeline
	{
		static constexpr int32_t UNROLL = 500 + 8 * 120;
		static constexpr int32_t TEXT_AT = UNROLL + 1000;
		static constexpr int32_t FADE = 1000;
		static constexpr int32_t BUTTONS_AT = TEXT_AT + 2000;
		static constexpr int32_t TIMEOUT_AT = TEXT_AT + 30000;

		// Since the window opened.
		int32_t elapsed = 0;

		// Moves on by `ms`; true on the step the time runs out.
		bool update(int32_t ms)
		{
			bool was_out = timed_out();
			elapsed += ms;

			return !was_out && timed_out();
		}

		bool text_shown() const
		{
			return elapsed >= TEXT_AT;
		}

		// The text's opacity, 0 to 1.
		float text_opacity() const
		{
			if (!text_shown())
				return 0.0f;

			return std::min(1.0f, static_cast<float>(elapsed - TEXT_AT) / FADE);
		}

		bool buttons_shown() const
		{
			return elapsed >= BUTTONS_AT;
		}

		bool timed_out() const
		{
			return elapsed >= TIMEOUT_AT;
		}
	};
}
