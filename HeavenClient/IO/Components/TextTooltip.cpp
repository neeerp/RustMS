//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "TextTooltip.h"

#include "MapTooltipLayout.h"

namespace ms
{
	namespace
	{
		// Where the text starts in the box, as in the map tooltips.
		const Point<int16_t> TEXT_POS = { map_tooltip::PADDING_X, 2 };
		constexpr int16_t MAX_WIDTH = 340;
	}

	TextTooltip::TextTooltip()
	{
		text = "";
	}

	void TextTooltip::draw(Point<int16_t> pos) const
	{
		if (text_label.empty())
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);
		text_label.draw(pos + TEXT_POS);
	}

	bool TextTooltip::set_text(std::string t)
	{
		if (text == t)
			return false;

		text = t;

		if (text.empty())
		{
			text_label = Text();
			return false;
		}

		text_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, text, MAX_WIDTH, true, tooltip::LINE_ADJ);

		map_tooltip::Size s = map_tooltip::size(text_label.width(), text_label.height(), 0, 0);
		size = Point<int16_t>(s.width, s.height);

		return true;
	}
}
