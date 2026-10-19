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
#pragma once

#include "Tooltip.h"

#include "../Gameplay/MapleMap/MapTooltipRules.h"
#include "../Graphics/Text.h"

#include <string>

namespace ms
{
	// The tooltips v83 draws on a canvas of its own (CUIToolTip, 0x8e7150 and
	// 0x8e6e7d) rather than from UI art: a translucent navy box with its
	// corner pixels left out. A map zone's title and description go 20
	// below the cursor, a portal's tip on the minimap 20 right of and below.
	class PlainTooltip : public Tooltip
	{
	public:
		PlainTooltip();

		// A map zone's title and description in the compact box.
		void set_zone(const std::string &title, const std::string &desc);
		// One line, as for a portal on the minimap.
		void set_line(const std::string &text);
		void reset();

		bool empty() const;

		// Draws the box with its top left at the given point.
		void draw_box(Point<int16_t> position) const;
		// Draws the box by the cursor, on the screen; the game's tooltips are
		// drawn 22 below it.
		void draw(Point<int16_t> position) const override;

	private:
		void set(const std::string &title, const std::string &desc, bool line);

		std::string title;
		std::string desc;
		bool line = false;
		Text title_label;
		Text desc_label;
		map_tooltips::Box box = {};
	};
}
