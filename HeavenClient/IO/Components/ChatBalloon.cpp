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
#include "ChatBalloon.h"

#include "../Constants.h"

#include <algorithm>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// ChatBalloon.img styles store their text color as a signed ARGB integer.
		Color argb_color(int32_t argb)
		{
			auto value = static_cast<uint32_t>(argb);
			auto channel = [value](int shift) { return ((value >> shift) & 0xFF) / 255.0f; };

			return Color(channel(16), channel(8), channel(0), 1.0f);
		}
	}

	ChatBalloon::ChatBalloon(int8_t type)
	{
		std::string typestr = type == -1 ? "dead" : std::to_string(std::max<int8_t>(type, 0));

		load(nl::nx::ui["ChatBalloon.img"][typestr], false);
	}

	ChatBalloon::ChatBalloon(const std::string &style, bool formatted)
	{
		load(nl::nx::ui["ChatBalloon.img"][style], formatted);
	}

	ChatBalloon::ChatBalloon() : ChatBalloon(0)
	{}

	void ChatBalloon::load(nl::node src, bool formatted)
	{
		constexpr const char *names[NUM_PIECES] = { "c", "n", "s", "w", "e", "nw", "ne", "sw", "se", "arrow" };

		for (size_t i = 0; i < NUM_PIECES; i++)
			pieces[i] = src[names[i]];

		xtile = std::max<int16_t>(pieces[NORTH].width(), 1);
		ytile = std::max<int16_t>(pieces[WEST].height(), 1);

		nl::node clr = src["clr"];
		textcolor = clr.data_type() == nl::node::type::integer ? argb_color(static_cast<int32_t>(clr.get_integer())) : Color(Color::Code::CBLACK);

		// The text itself is white so the style color applied at draw time is exact.
		textlabel = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE, "", WRAP_WIDTH, formatted);

		duration = 0;
	}

	void ChatBalloon::change_text(const std::string &text)
	{
		textlabel.change_text(text);

		duration = DURATION + FADE;
	}

	void ChatBalloon::draw(Point<int16_t> position) const
	{
		if (duration == 0)
			return;

		float opacity = duration < FADE ? static_cast<float>(duration) / FADE : 1.0f;

		// The content area fits the text exactly. The border pieces are drawn
		// outside it through their asset origins, with the edges stretched to
		// the content size, so the text always sits inside the frame.
		int16_t width = std::max<int16_t>(textlabel.width() + 2 * PADDING, xtile);
		int16_t height = std::max<int16_t>(textlabel.height() - LINE_GAP, ytile);
		int16_t left = position.x() - width / 2;
		int16_t right = left + width;
		int16_t bottom = position.y();
		int16_t top = bottom - height;

		auto at = [opacity](int16_t x, int16_t y) { return DrawArgument(Point<int16_t>(x, y), opacity); };
		auto stretched = [opacity](int16_t x, int16_t y, int16_t w, int16_t h)
		{
			Point<int16_t> p(x, y);

			return DrawArgument(p, p, Point<int16_t>(w, h), 1.0f, 1.0f, opacity, 0.0f);
		};

		pieces[CENTER].draw(stretched(left, top, width, height));
		pieces[NORTH].draw(stretched(left, top, width, 0));
		pieces[SOUTH].draw(stretched(left, bottom, width, 0));
		pieces[WEST].draw(stretched(left, top, 0, height));
		pieces[EAST].draw(stretched(right, top, 0, height));

		pieces[NORTHWEST].draw(at(left, top));
		pieces[NORTHEAST].draw(at(right, top));
		pieces[SOUTHWEST].draw(at(left, bottom));
		pieces[SOUTHEAST].draw(at(right, bottom));
		pieces[ARROW].draw(at(position.x(), bottom));

		Color color(textcolor.r(), textcolor.g(), textcolor.b(), opacity);

		// Each layout line includes leading below the glyphs; shift the text up so
		// the padding above and below it is even.
		int16_t text_top = top - LINE_GAP;
		textlabel.draw(DrawArgument(Point<int16_t>(position.x(), text_top), color));
	}

	void ChatBalloon::update()
	{
		duration -= Constants::TIMESTEP;

		if (duration < 0)
			duration = 0;
	}

	void ChatBalloon::expire()
	{
		duration = 0;
	}

	bool ChatBalloon::is_active() const
	{
		return duration > 0;
	}

	ChatBalloonHorizontal::ChatBalloonHorizontal()
	{
		nl::node Balloon = nl::nx::ui["Login.img"]["WorldNotice"]["Balloon"];

		arrow = Balloon["arrow"];
		center = Balloon["c"];
		east = Balloon["e"];
		northeast = Balloon["ne"];
		north = Balloon["n"];
		northwest = Balloon["nw"];
		west = Balloon["w"];
		southwest = Balloon["sw"];
		south = Balloon["s"];
		southeast = Balloon["se"];

		xtile = std::max<int16_t>(north.width(), 1);
		ytile = std::max<int16_t>(west.height(), 1);

		textlabel = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK, "", 300);
	}

	void ChatBalloonHorizontal::draw(Point<int16_t> position) const
	{
		int16_t width = textlabel.width() + 9;
		int16_t height = textlabel.height() - 2;

		int16_t left = position.x() - width / 2;
		int16_t top = position.y() - height;
		int16_t right = left + width;
		int16_t bottom = top + height;

		northwest.draw(DrawArgument(left, top));
		southwest.draw(DrawArgument(left, bottom));

		for (int16_t y = top; y < bottom; y += ytile)
		{
			west.draw(DrawArgument(left, y));
			east.draw(DrawArgument(right, y));
		}

		center.draw(DrawArgument(Point<int16_t>(left - 8, top), Point<int16_t>(width + 8, height)));

		for (int16_t x = left; x < right; x += xtile)
		{
			north.draw(DrawArgument(x, top));
			south.draw(DrawArgument(x, bottom));
		}

		northeast.draw(DrawArgument(right, top));
		southeast.draw(DrawArgument(right, bottom));

		arrow.draw(DrawArgument(right + 1, top));
		textlabel.draw(DrawArgument(left + 6, top - 5));
	}

	void ChatBalloonHorizontal::change_text(const std::string &text)
	{
		textlabel.change_text(text);
	}
}