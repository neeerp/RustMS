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
		set(t, d, Kind::ZONE);
		anchor.reset();
	}

	void PlainTooltip::set_line(const std::string &text)
	{
		set(text, "", Kind::LINE);
		anchor.reset();
	}

	// Show (0x4c131c) takes an empty description for the one-line box,
	// which has no use for above.
	void PlainTooltip::set_help(const std::string &t, const std::string &d, std::optional<Point<int16_t>> a, bool ab)
	{
		set(t, d, Kind::HELP);
		anchor = a;
		above = ab;
	}

	void PlainTooltip::reset()
	{
		set("", "", Kind::ZONE);
		anchor.reset();
	}

	void PlainTooltip::set(const std::string &t, const std::string &d, Kind k)
	{
		if (t == title && d == desc && k == kind)
			return;

		title = t;
		desc = d;
		kind = k;
		line = kind == Kind::LINE || (kind == Kind::HELP && desc.empty());
		bool help = kind == Kind::HELP && !line;
		title_label = Text(help ? Text::Font::A12B : Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
						   title, 0, false);
		int16_t title_width = title.empty() ? 0 : title_label.width();

		if (line)
		{
			desc_label = Text();
			box = map_tooltips::line_box(title_width);

			return;
		}

		// v83 writes the help's description in white (font 7, FUN_008e49b5).
		if (help)
		{
			box = map_tooltips::help_box(title_width, 0);
			desc_label = Text();

			if (!desc.empty())
			{
				desc_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, desc, box.wrap, true,
								  tooltip::LINE_ADJ);
				box = map_tooltips::help_box(title_width, desc_label.height() - tooltip::LINE_ADJ);
			}

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
		auto fill = [&](int16_t left, int16_t top, int16_t width, int16_t height)
		{
			gl.drawrectangle(x + left, y + top, width, height, 0.0f, 0.0f, FILL_BLUE, FILL_ALPHA);
		};

		fill(1, 0, w - 2, 1);
		fill(1, h - 1, w - 2, 1);

		// The help also leaves out a ring one pixel inside the edge (the
		// layer's fourth flag, 0x8f35b6), all but its corners.
		if (kind == Kind::HELP && !line)
		{
			fill(0, 1, 2, 1);
			fill(w - 2, 1, 2, 1);
			fill(0, 2, 1, h - 4);
			fill(2, 2, w - 4, h - 4);
			fill(w - 1, 2, 1, h - 4);
			fill(0, h - 2, 2, 1);
			fill(w - 2, h - 2, 2, 1);
		} else
		{
			fill(0, 1, w, h - 2);
		}

		title_label.draw(position + Point<int16_t>(box.title_x, text_y(box.title_y)));
		desc_label.draw(position + Point<int16_t>(box.desc_x, text_y(box.desc_y)));
	}

	void PlainTooltip::draw(Point<int16_t> position) const
	{
		if (empty())
			return;

		Point<int16_t> at = position - Point<int16_t>(0, DRAWN_BELOW) + (line ? LINE_OFFSET : ZONE_OFFSET);

		if (anchor)
			at = *anchor - Point<int16_t>(0, above && !line ? box.height : 0);

		auto [x, y] = map_tooltips::clamp_screen(at.x(), at.y(), box.width, box.height,
												 Constants::Constants::get().get_viewwidth(),
												 Constants::Constants::get().get_viewheight());
		draw_box(Point<int16_t>(x, y));
	}
}
