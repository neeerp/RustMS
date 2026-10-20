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
#include <cstdint>

#include "Tooltip.h"
#include "TooltipBox.h"

#include "../Graphics/Text.h"

namespace ms
{
	// The v83 item tooltip: the name, and the icon with the description beside
	// it.
	class ItemTooltip : public Tooltip
	{
	public:
		ItemTooltip();

		void draw(Point<int16_t> position) const override;

		// An item in the inventory also has its expiry and flags, and is `owned`. A pet's expiry is when its Water
		// of Life dries up, and one with limitedLife has `remain_life` seconds left.
		bool set_item(int32_t itemid, int64_t expiration = 0, int16_t flags = 0, bool owned = false,
					  int32_t remain_life = 0);

	private:
		int32_t itemid;
		int64_t expiration;
		int16_t flags;
		bool owned;
		int32_t remain_life;
		Point<int16_t> size;

		TooltipBox box;
		TooltipIcon icon;

		Texture itemicon;
		Text name;
		// The orange lines under the name.
		std::array<Text, 2> properties;
		int16_t properties_height;
		Texture expiry_dot;
		Text expiry;
		int16_t expiry_height;
		Text desc;
	};
}
