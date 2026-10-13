#pragma once

#include <cstdint>

namespace ms
{
	// Which of a dialog page's two answers (YES/NO, ACCEPT/DECLINE) the
	// keyboard has focused. Left focuses the left one and Right the right
	// one; Enter presses the focused one, or the left one when neither is.
	namespace answer_focus
	{
		enum Focus : int8_t
		{
			NONE = -1,
			LEFT,
			RIGHT
		};

		inline Focus move(Focus focus, bool left, bool right)
		{
			if (left)
				return Focus::LEFT;

			if (right)
				return Focus::RIGHT;

			return focus;
		}

		inline Focus pressed(Focus focus)
		{
			return focus == Focus::NONE ? Focus::LEFT : focus;
		}
	}
}
