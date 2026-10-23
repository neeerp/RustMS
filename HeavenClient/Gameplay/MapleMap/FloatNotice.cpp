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
#include "FloatNotice.h"

#include "../WeatherRules.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// How far below the art's text line our text's top sits.
		constexpr int16_t TEXT_TOP = -6;
	}

	bool FloatNotice::show(const std::string &msg, int32_t float_type)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["FloatNotice"][std::to_string(float_type)];

		if (!src)
			return false;

		left = src["0"];
		tile = src["1"];
		right = src["2"];
		y = src["y"];
		text = Text(Text::Font::A14B, Text::Alignment::LEFT, Color::Name::WHITE, msg);
		active = true;

		return true;
	}

	void FloatNotice::clear()
	{
		active = false;
	}

	bool FloatNotice::shown() const
	{
		return active;
	}

	void FloatNotice::draw() const
	{
		if (!active)
			return;

		int16_t view_width = Constants::Constants::get().get_viewwidth();
		weather::Bar bar = weather::bar(view_width, left.width(), tile.width(), right.width(), text.width());

		// The art's origins are ignored: each piece's top left goes at its spot.
		auto piece = [&](const Texture &texture, int16_t x) {
			Point<int16_t> left_top(static_cast<int16_t>(bar.left + x), weather::BAR_TOP);
			texture.draw(DrawArgument(left_top + texture.get_origin(), weather::BAR_ALPHA));
		};

		piece(left, 0);

		for (int16_t i = 0; i < bar.tiles; i++)
			piece(tile, static_cast<int16_t>(left.width() + i * tile.width()));

		piece(right, static_cast<int16_t>(left.width() + bar.tiles * tile.width()));

		text.draw(Point<int16_t>(static_cast<int16_t>(bar.left + bar.text_x),
								 static_cast<int16_t>(weather::BAR_TOP + y + TEXT_TOP)));
	}
}
