#pragma once

#include <cstdint>

// Where v83 draws the chat log (SetMode 0x8d4a6d, drawn by 0x8dbb93), from the status bar's top. The log is a
// 580 pixel wide layer, as tall as its mode says, whose rows of 13 pixels stack up from its bottom.
namespace ms::chat_log
{
	enum class Mode
	{
		// The latest line, in the strip
		LINE,
		// That line above the strip, while typing
		TYPING,
		// The expanded log, as tall as it was left
		EXPANDED
	};

	constexpr int16_t ROW = 13;

	// The layer's top: 0x1fd - H in the status bar's window, whose top is 507 above the bar's, so the expanded log
	// overlaps the bar by 2 pixels. The one-line modes put it at 0x1e6 and 0x201.
	inline int16_t top(Mode mode, int16_t height)
	{
		switch (mode)
		{
			case Mode::EXPANDED:
				return 2 - height;
			case Mode::TYPING:
				return -21;
			default:
				return 6;
		}
	}

	// How far above its row a line's text sits: the one-line modes draw it 5 pixels up (0x8dd054).
	inline int16_t lift(Mode mode)
	{
		return mode == Mode::EXPANDED ? 0 : 5;
	}

	// A highlighted row's strip (0x8dc0ae), from the layer's top, for row `i` from the bottom: a pixel above the
	// row in the expanded log, and from 10 above it to the layer's bottom in the others. It stops short of the
	// scroll bar by 14 pixels.
	struct Strip
	{
		int16_t top;
		int16_t height;
	};

	inline Strip strip(Mode mode, int16_t height, int16_t i)
	{
		int16_t row = height - ROW * (i + 1);

		if (mode == Mode::EXPANDED)
			return { static_cast<int16_t>(row - 1), ROW };

		auto y = static_cast<int16_t>(row - 10);

		return { y, static_cast<int16_t>(height - y) };
	}

	inline int16_t strip_width(int16_t arrow_width)
	{
		return 566 - arrow_width;
	}
}
