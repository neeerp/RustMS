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
#include "MapleButton.h"

#include <algorithm>

namespace ms
{
	MapleButton::MapleButton(nl::node src, Point<int16_t> pos)
	{
		nl::node normal = src["normal"];

		if (normal.size() > 1)
			animations[Button::State::NORMAL] = normal;
		else
			textures[Button::State::NORMAL] = normal["0"];

		textures[Button::State::PRESSED] = src["pressed"]["0"];
		textures[Button::State::MOUSEOVER] = src["mouseOver"]["0"];
		textures[Button::State::DISABLED] = src["disabled"]["0"];
		focus = src["keyFocused"]["0"];

		// v83 sizes a button once, when it is made, to the largest frame of
		// its normal, pressed, disabled and mouseOver states (CCtrlButton::
		// CreateCtrl, 4c007a), and a change of state never resizes it. So a
		// press shown by a smaller frame, as the race select's 1x1, still
		// takes the release.
		for (const char *name : { "normal", "pressed", "disabled", "mouseOver" })
		{
			for (nl::node frame : src[name])
			{
				Point<int16_t> dimensions = Texture(frame).get_dimensions();
				hitsize = Point<int16_t>(
						std::max(hitsize.x(), dimensions.x()),
						std::max(hitsize.y(), dimensions.y())
				);
			}
		}

		hitorigin = textures[Button::State::NORMAL].is_valid()
			? textures[Button::State::NORMAL].get_origin()
			: animations[Button::State::NORMAL].get_origin();

		active = true;
		position = pos;
		state = Button::State::NORMAL;
	}

	MapleButton::MapleButton(nl::node src, int16_t x, int16_t y) : MapleButton(src, Point<int16_t>(x, y))
	{}

	MapleButton::MapleButton(nl::node src) : MapleButton(src, Point<int16_t>())
	{}

	void MapleButton::draw(Point<int16_t> parentpos) const
	{
		if (active)
		{
			DrawArgument args = greyed && state == Button::State::DISABLED
				? DrawArgument(position + parentpos, Color::grayscale(opacity))
				: DrawArgument(position + parentpos, opacity);
			textures[state].draw(args);
			animations[state].draw(args, 1.0f);

			if (focused)
				focus.draw(position + parentpos);
		}
	}

	void MapleButton::update()
	{
		if (active)
			animations[state].update(6);
	}

	void MapleButton::set_opacity(float value)
	{
		opacity = value;
	}

	void MapleButton::grey_when_disabled()
	{
		greyed = true;
	}

	Rectangle<int16_t> MapleButton::bounds(Point<int16_t> parentpos) const
	{
		Point<int16_t> lt = parentpos + position - hitorigin;

		return Rectangle<int16_t>(lt, lt + hitsize);
	}

	int16_t MapleButton::width() const
	{
		return textures[state].width();
	}

	Point<int16_t> MapleButton::origin() const
	{
		return textures[state].get_origin();
	}
}