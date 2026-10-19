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

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The medal name tag v83 builds under a character's name tags
	// (FUN_005f0334, type 0x3ee) from UI NameTag.img/medal/<medalTag>: the
	// left cap `w`, the tile `c` repeated under the text, and the right cap
	// `e`.
	namespace MedalTagLayout
	{
		// A canvas's size and origin.
		struct Piece
		{
			int16_t width;
			int16_t height;
			int16_t cx;
			int16_t cy;
		};

		struct Spot
		{
			int16_t x;
			int16_t y;
		};

		// Where everything goes, from the tag's top-left corner.
		struct Layout
		{
			int16_t width;
			int16_t height;
			// The x of each tile; they all sit at tile_y.
			std::vector<int16_t> tiles;
			int16_t tile_y;
			Spot left;
			Spot right;
			// The top-left of the text.
			Spot text;
			// The tag is centred on the character at this x.
			int16_t origin_x;
		};

		// The tiles cover the text in whole tiles, and each piece sits on a
		// shared line at the deepest cy. The right cap ignores its cy and
		// goes at the top.
		inline Layout layout(Piece w, Piece c, Piece e, int16_t text_width)
		{
			int32_t tile = std::max<int32_t>(c.width, 1);
			int32_t middle = (tile + text_width - 1) / tile * tile;
			int32_t width = w.cx + middle + (e.width - e.cx);
			int32_t line = std::max({ w.cy, c.cy, e.cy });
			int32_t below = std::max({ w.height - w.cy, c.height - c.cy, e.height - e.cy });

			Layout result;
			result.width = static_cast<int16_t>(width);
			result.height = static_cast<int16_t>(below + line);
			result.tile_y = static_cast<int16_t>(line - c.cy);

			for (int32_t x = 0; x < middle; x += tile)
				result.tiles.push_back(static_cast<int16_t>(w.cx + x));

			result.left = { 0, static_cast<int16_t>(line - w.cy) };
			result.right = { static_cast<int16_t>(width - e.width), 0 };
			result.text = { static_cast<int16_t>((middle - text_width) / 2 + w.cx), static_cast<int16_t>(line - 2) };
			result.origin_x = static_cast<int16_t>(width / 2);

			return result;
		}

		// The medal's name without "'s Medal", which v83 strips for the tag.
		inline std::string text(std::string name)
		{
			const std::string suffix = "'s Medal";
			size_t found = name.find(suffix);

			if (found != std::string::npos)
				name.erase(found, suffix.size());

			return name;
		}
	}
}
