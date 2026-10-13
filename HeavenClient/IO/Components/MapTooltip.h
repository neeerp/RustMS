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
#include "MapleFrame.h"
#include "MapTooltipLayout.h"

#include "../Graphics/Text.h"
#include "../Graphics/Geometry.h"

#include <vector>

namespace ms
{
	class MapTooltip : public Tooltip
	{
	public:
		MapTooltip();

		void draw(Point<int16_t> position) const override;

		// Shows a map, or a spot on one, rebuilt only when any of it changes.
		void set_map(Tooltip::Parent parent, std::string name, std::string description, int32_t mapid, bool bolded);

		void reset();

	private:
		static constexpr uint8_t BOTTOM_PADDING = map_tooltip::BOTTOM_PADDING;
		static constexpr Point<int16_t> SEPARATOR_ADJ = Point<int16_t>(5, 0);
		static constexpr Point<int16_t> LIFE_LABEL_ADJ = Point<int16_t>(20, 3);
		static constexpr Point<int16_t> LIFE_ICON_ADJ = Point<int16_t>(5, 9);

		MapleFrame frame;

		Texture cover;
		Texture Mob;
		Texture Npc;
		Texture Party;

		Tooltip::Parent parent = Tooltip::Parent::NONE;
		std::string name;
		std::string description;
		int32_t mapid = 0;
		bool bolded = false;

		Text name_label;
		Text name_simple;
		Text desc_label;
		Text desc_simple;
		std::vector<Text> mob_labels;
		std::vector<Text> npc_labels;

		int16_t fillwidth;
		int16_t fillheight;

		ColorLine separator;
	};
}