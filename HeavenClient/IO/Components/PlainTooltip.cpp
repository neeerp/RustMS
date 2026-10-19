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
#include "PlainTooltip.h"

#include "TooltipBox.h"

#include "../Constants.h"

#include "../../Graphics/GraphicsGL.h"

namespace ms
{
	namespace
	{
		// How far the cursor the game's tooltips get is below the real one.
		constexpr int16_t DRAWN_BELOW = 22;
		const Point<int16_t> ZONE_OFFSET = { 0, 20 }, LINE_OFFSET = { 20, 20 };

		// v83 fills the box with 0xA0000040.
		constexpr float FILL_BLUE = 0x40 / 255.0f, FILL_ALPHA = 0xA0 / 255.0f;

		// How far down to draw text whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + map_tooltips::BASELINE - line;
		}
	}

	PlainTooltip::PlainTooltip() {}

	void PlainTooltip::set_zone(const std::string &t, const std::string &d)
	{
		set(t, d, false);
	}

	void PlainTooltip::set_line(const std::string &text)
	{
		set(text, "", true);
	}

	void PlainTooltip::reset()
	{
		set("", "", false);
	}

	void PlainTooltip::set(const std::string &t, const std::string &d, bool l)
	{
		if (t == title && d == desc && l == line)
			return;

		title = t;
		desc = d;
		line = l;
		title_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, title, 0, false);
		int16_t title_width = title.empty() ? 0 : title_label.width();

		if (line)
		{
			desc_label = Text();
			box = map_tooltips::line_box(title_width);

			return;
		}

		box = map_tooltips::compact_box(title_width, 0);
		desc_label = Text();

		// The description wraps inside the title's width, its lines 16
		// apart.
		if (!desc.empty() && box.width > 0)
		{
			desc_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::TIPORANGE, desc, box.wrap, true,
							  tooltip::LINE_ADJ);
			box = map_tooltips::compact_box(title_width, desc_label.height() - tooltip::LINE_ADJ);
		}
	}

	bool PlainTooltip::empty() const
	{
		return box.width <= 0 || (title.empty() && desc.empty());
	}

	void PlainTooltip::draw_box(Point<int16_t> position) const
	{
		if (empty())
			return;

		int16_t x = position.x(), y = position.y(), w = box.width, h = box.height;
		GraphicsGL &gl = GraphicsGL::get();
		gl.drawrectangle(x + 1, y, w - 2, 1, 0.0f, 0.0f, FILL_BLUE, FILL_ALPHA);
		gl.drawrectangle(x, y + 1, w, h - 2, 0.0f, 0.0f, FILL_BLUE, FILL_ALPHA);
		gl.drawrectangle(x + 1, y + h - 1, w - 2, 1, 0.0f, 0.0f, FILL_BLUE, FILL_ALPHA);

		title_label.draw(position + Point<int16_t>(box.title_x, text_y(box.title_y)));
		desc_label.draw(position + Point<int16_t>(box.desc_x, text_y(box.desc_y)));
	}

	void PlainTooltip::draw(Point<int16_t> position) const
	{
		if (empty())
			return;

		Point<int16_t> at = position - Point<int16_t>(0, DRAWN_BELOW) + (line ? LINE_OFFSET : ZONE_OFFSET);
		auto [x, y] = map_tooltips::clamp_screen(at.x(), at.y(), box.width, box.height,
												 Constants::Constants::get().get_viewwidth(),
												 Constants::Constants::get().get_viewheight());
		draw_box(Point<int16_t>(x, y));
	}
}
