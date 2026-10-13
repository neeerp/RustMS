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

#include <string>

#include <cstdint>

#include "Tooltip.h"
#include "TooltipBox.h"

#include "../Graphics/Text.h"

namespace ms
{
	// The name of a map, a spot on the world map, or something on the
	// minimap, with its description if it has one, in the v83 tooltip box.
	class MapTooltip : public Tooltip
	{
	public:
		MapTooltip();

		void draw(Point<int16_t> position) const override;

		// Shows a name, rebuilt only when it or the description changes.
		void set_map(Tooltip::Parent parent, std::string name, std::string description);

		void reset();

	private:
		TooltipBox box;

		Tooltip::Parent parent = Tooltip::Parent::NONE;
		std::string name;
		std::string description;

		Text name_label;
		Text desc_label;

		Point<int16_t> size;
	};
}
