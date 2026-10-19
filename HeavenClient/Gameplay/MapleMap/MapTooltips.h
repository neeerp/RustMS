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

#include "MapTooltipRules.h"

#include "../../IO/Components/PlainTooltip.h"
#include "../../Template/Point.h"

#include <nlnx/node.hpp>

#include <map>
#include <string>

namespace ms
{
	// A map's ToolTip zones (v83 ZoneSet, FUN_008f622c): a numbered zone's
	// String ToolTipHelp.img/Mapobject text shows by the cursor while it is
	// over the zone, and in the map, at the zone's bottom right, while the
	// player stands in its "<N>char" zone.
	class MapTooltips
	{
	public:
		MapTooltips(int32_t mapid, nl::node src);
		// A map without zones.
		MapTooltips();

		// Shows the tip of the zone under the cursor, given in map
		// coordinates (FUN_0094cfc3).
		void hover(Point<int16_t> cursor);
		// Takes the hover tip down, as when something else has the cursor.
		void leave();

		// Puts up and takes down the tips of the zones the player stands
		// in (FUN_0094a144).
		void update(Point<int16_t> player);

		void draw(Point<int16_t> viewpos) const;

	private:
		struct ZoneText
		{
			std::string title;
			std::string desc;
		};

		struct Balloon
		{
			Point<int16_t> position;
			PlainTooltip tip;
		};

		std::map<int32_t, map_tooltips::Zone> zones;
		std::map<int32_t, map_tooltips::Zone> char_zones;
		std::map<int32_t, ZoneText> texts;
		std::map<int32_t, Balloon> balloons;
		// The zone under the cursor, whose balloon makes way for the hover
		// tip, or -1.
		int32_t hovered = -1;
	};
}
