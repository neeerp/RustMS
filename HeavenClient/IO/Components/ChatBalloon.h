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
#pragma once

#include <array>
#include <string>

#include <cstdint>

#include "../Graphics/Texture.h"

#include "../Graphics/Text.h"

#include <nlnx/node.hpp>

namespace ms
{
	class ChatBalloon
	{
	public:
		// Player balloon in a numbered UI.nx/ChatBalloon.img style (0 is the default).
		ChatBalloon(int8_t type);

		// Balloon in a named style such as "npc"; NPC lines keep their #b/#k markup.
		ChatBalloon(const std::string &style, bool formatted);

		ChatBalloon();

		// Draws the balloon so that its arrow hangs down to the given point.
		void draw(Point<int16_t> position) const;

		void update();

		void change_text(const std::string &text);

		void expire();

		bool is_active() const;

	private:
		void load(nl::node src, bool formatted);

		// v83 CUser::OnChat shows a balloon for 5 s, then fades it out over 1 s.
		static constexpr int16_t DURATION = 5000;
		static constexpr int16_t FADE = 1000;
		// Lines wrap at about 80 px, which matches the narrow balloons of the original.
		static constexpr uint16_t WRAP_WIDTH = 80;
		static constexpr int16_t PADDING = 2;
		static constexpr int16_t LINE_GAP = 3;

		// The frame pieces, named in ChatBalloon.img as c, n, s, w, e, nw, ne, sw, se, and arrow.
		enum Piece : uint8_t
		{
			CENTER, NORTH, SOUTH, WEST, EAST, NORTHWEST, NORTHEAST, SOUTHWEST, SOUTHEAST, ARROW, NUM_PIECES
		};

		std::array<Texture, NUM_PIECES> pieces;
		int16_t xtile = 1;
		int16_t ytile = 1;
		Color textcolor;
		Text textlabel;
		int16_t duration;
	};

	class ChatBalloonHorizontal
	{
	public:
		ChatBalloonHorizontal();

		void draw(Point<int16_t> position) const;

		void change_text(const std::string &text);

	private:
		Text textlabel;
		Texture arrow;
		Texture center;
		Texture east;
		Texture northeast;
		Texture north;
		Texture northwest;
		Texture west;
		Texture southwest;
		Texture south;
		Texture southeast;
		int16_t xtile;
		int16_t ytile;
	};
}