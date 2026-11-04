#pragma once

#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>

namespace ms
{
	// `text` cut to `width`, with `tail` after it if cut (FUN_00988cf3).
	inline std::string fit_text(Text::Font font, const std::string& text, int16_t width, const std::string& tail = "..")
	{
		Text measure(font, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);

		if (measure.width() <= width)
			return text;

		std::string cut = text;

		while (!cut.empty())
		{
			cut.pop_back();
			measure.change_text(cut + tail);

			if (measure.width() <= width)
				break;
		}

		return cut + tail;
	}
}
