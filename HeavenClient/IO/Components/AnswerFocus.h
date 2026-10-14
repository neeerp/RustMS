#pragma once

#include <cstdint>

namespace ms
{
	// Which of a dialog page's two answers (YES/NO, ACCEPT/DECLINE) the
	// keyboard has focused. As in the original, the first arrow of either
	// kind focuses the left one; then Left and Right move between them.
	// Enter presses the focused one, or the left one when neither is.
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
			if (focus == Focus::NONE && (left || right))
				return Focus::LEFT;

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
