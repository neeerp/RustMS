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
#include "../../Util/NxFiles.h"

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

		// The v83 ChatBalloon.img, or the modern one for the later styles
		// that UIClassic.nx doesn't have.
		nl::node balloons(const std::string &style)
		{
			nl::node classic = NxFiles::ui_classic["ChatBalloon.img"];

			return classic.resolve(style) ? classic : nl::nx::ui["ChatBalloon.img"];
		}

		// Some styles reuse another style's pieces through "_inlink" (a path
		// within ChatBalloon.img) instead of storing the bitmap again.
		nl::node piece(nl::node file, nl::node style, const char *name)
		{
			nl::node node = style[name];
			std::string link = node["_inlink"];

			return link.empty() ? node : file.resolve(link);
		}
	}

	ChatBalloon::ChatBalloon(int8_t type)
	{
		std::string typestr = type == -1 ? "dead" : std::to_string(std::max<int8_t>(type, 0));

		load(typestr, false, WRAP_WIDTH);
	}

	ChatBalloon::ChatBalloon(const std::string &style, bool formatted, uint16_t wrap_width)
	{
		load(style, formatted, wrap_width);
	}

	ChatBalloon::ChatBalloon() : ChatBalloon(0)
	{}

	ChatBalloon ChatBalloon::pet(int32_t style)
	{
		std::string path = "pet/" + std::to_string(style);
		ChatBalloon balloon;

		balloon.for_pet = true;
		balloon.missing = !balloons(path).resolve(path);
		balloon.load(path, false, WRAP_WIDTH);

		nl::node file = balloons(path);
		balloon.head = piece(file, file.resolve(path), "head");

		// Lines wrap at 14 tiles of the top edge (0x489942).
		balloon.textlabel = Text(Text::Font::BALLOON, Text::Alignment::CENTER, Color::Name::WHITE, "", 14 * balloon.xtile, false);

		for (Piece under : { SOUTH, SOUTHWEST, SOUTHEAST, ARROW })
		{
			const Texture &texture = balloon.pieces[under];
			balloon.below = std::max<int16_t>(balloon.below, texture.height() - texture.get_origin().y());
		}

		return balloon;
	}

	void ChatBalloon::load(const std::string &style, bool formatted, uint16_t wrap_width)
	{
		nl::node file = balloons(style);
		nl::node src = file.resolve(style);

		constexpr const char *names[NUM_PIECES] = { "c", "n", "s", "w", "e", "nw", "ne", "sw", "se", "arrow" };

		for (size_t i = 0; i < NUM_PIECES; i++)
			pieces[i] = piece(file, src, names[i]);

		xtile = std::max<int16_t>(pieces[NORTH].width(), 1);
		ytile = std::max<int16_t>(pieces[WEST].height(), 1);

		nl::node clr = src["clr"];
		textcolor = clr.data_type() == nl::node::type::integer ? argb_color(static_cast<int32_t>(clr.get_integer())) : Color(Color::Code::CBLACK);

		// The text itself is white so the style color applied at draw time is exact.
		textlabel = Text(Text::Font::BALLOON, Text::Alignment::CENTER, Color::Name::WHITE, "", wrap_width, formatted);

		duration = 0;
	}

	void ChatBalloon::change_text(const std::string &text)
	{
		textlabel.change_text(text);

		// A pet's balloon goes at once (FUN_0048e6d4; only a user's fades).
		duration = missing ? 0 : for_pet ? DURATION : DURATION + FADE;
	}

	void ChatBalloon::draw(Point<int16_t> position) const
	{
		if (duration == 0)
			return;

		if (for_pet)
		{
			draw_pet(position);
			return;
		}

		float opacity = duration < FADE ? static_cast<float>(duration) / FADE : 1.0f;

		// The content area is exactly the text block, one 14 px row per line, as
		// in the original. The border pieces are drawn outside it through their
		// asset origins, with the edges stretched to the content size.
		int16_t width = std::max<int16_t>(textlabel.width(), xtile);
		int16_t height = std::max<int16_t>(textlabel.height(), ytile);
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

		int16_t text_top = top - TEXT_DESCENT;
		textlabel.draw(DrawArgument(Point<int16_t>(position.x(), text_top), color));
	}

	void ChatBalloon::draw_pet(Point<int16_t> tip) const
	{
		// FUN_004890c4: an odd number of whole tiles, the arrow replacing
		// the middle bottom one and the head the middle top one. The layer's
		// bottom is the arrow tip, its left edge 3 - inner / 2 - R off the pet.
		int16_t tiles = static_cast<int16_t>((textlabel.width() + xtile - 1) / xtile) | 1;
		int16_t width = tiles * xtile;
		int16_t height = std::max<int16_t>(textlabel.height(), ytile);
		int16_t left = tip.x() + 3 - width / 2;
		int16_t right = left + width;
		int16_t bottom = tip.y() - below;
		int16_t top = bottom - height;
		int16_t middle = left + tiles / 2 * xtile;

		auto stretched = [](int16_t x, int16_t y, int16_t w, int16_t h)
		{
			Point<int16_t> p(x, y);

			return DrawArgument(p, p, Point<int16_t>(w, h), 1.0f, 1.0f, 1.0f, 0.0f);
		};

		pieces[CENTER].draw(stretched(left, top, width, height));
		pieces[WEST].draw(stretched(left, top, 0, height));
		pieces[EAST].draw(stretched(right, top, 0, height));

		for (int16_t i = 0; i < tiles; i++)
		{
			int16_t x = left + i * xtile;

			if (x != middle || !head.is_valid())
				pieces[NORTH].draw(DrawArgument(x, top));

			if (x != middle || !pieces[ARROW].is_valid())
				pieces[SOUTH].draw(DrawArgument(x, bottom));
		}

		head.draw(DrawArgument(middle, top));
		pieces[ARROW].draw(DrawArgument(middle, bottom));

		pieces[NORTHWEST].draw(DrawArgument(left, top));
		pieces[NORTHEAST].draw(DrawArgument(right, top));
		pieces[SOUTHWEST].draw(DrawArgument(left, bottom));
		pieces[SOUTHEAST].draw(DrawArgument(right, bottom));

		textlabel.draw(DrawArgument(Point<int16_t>(left + width / 2, top - TEXT_DESCENT), textcolor));
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
		nl::node Balloon = NxFiles::ui_classic["Login.img"]["WorldNotice"]["Balloon"];

		arrow = Balloon["slArrow"];
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
		// The edge pieces sit outside the box, which hugs the text.
		int16_t width = textlabel.width() + 12;
		int16_t height = textlabel.height() - 5;

		int16_t left = position.x() - width / 2;
		int16_t top = position.y() - height;
		int16_t right = left + width;
		int16_t bottom = top + height;

		center.draw(DrawArgument(Point<int16_t>(left, top), Point<int16_t>(width, height)));

		for (int16_t y = top; y < bottom; y += ytile)
		{
			west.draw(DrawArgument(left, y));
			east.draw(DrawArgument(right, y));
		}

		// The tail replaces the bottom edge at the left end.
		for (int16_t x = left; x < right; x += xtile)
		{
			north.draw(DrawArgument(x, top));

			if (x >= left + arrow.width())
				south.draw(DrawArgument(x, bottom));
		}

		northwest.draw(DrawArgument(left, top));
		northeast.draw(DrawArgument(right, top));
		southwest.draw(DrawArgument(left, bottom));
		southeast.draw(DrawArgument(right, bottom));

		arrow.draw(DrawArgument(left, bottom));
		textlabel.draw(DrawArgument(left + 6, top - 6));
	}

	void ChatBalloonHorizontal::change_text(const std::string &text)
	{
		textlabel.change_text(text);
	}
}