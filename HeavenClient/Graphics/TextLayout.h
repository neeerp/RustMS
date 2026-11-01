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

#include "Text.h"

#include <functional>

namespace ms
{
	// Lay out a text in a font whose lines are `linespace` apart and whose
	// characters move the pen by `advance`, given whether they are bold. The n-th Text::BOX character takes
	// the n-th of `boxes` (width, height). Positions in the layout are byte
	// offsets into `text` (UTF-8). A `tooltip` text reads '#' as v83's
	// tooltips do (0x8e40e4) rather than as NPC dialogue does.
	Text::Layout layout_text(const std::string &text, int16_t linespace,
							 const std::function<int16_t(char32_t, bool)> &advance, Text::Alignment alignment,
							 int16_t maxwidth, bool formatted, int16_t line_adj,
							 const std::vector<Point<int16_t>> &boxes, bool tooltip = false);
}
