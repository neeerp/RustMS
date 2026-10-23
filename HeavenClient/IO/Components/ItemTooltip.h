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
#include <string>

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

		// A real item also has its expiry, flags and `owner` (who made it), and is `owned`. A pet's expiry is when
		// its Water of Life dries up, and one with limitedLife has `remain_life` seconds left.
		bool set_item(int32_t itemid, int64_t expiration = 0, int16_t flags = 0, bool owned = false,
					  int32_t remain_life = 0, const std::string &owner = {});
		// A Cash Shop commodity lasting `period` days, 0 for ever, as Item.nx makes it: the period for a date, and
		// a pet's whole life (FUN_004baf1a). A `desc` replaces Item.nx's description.
		void set_commodity(int32_t itemid, int32_t period, const std::string &desc = {});
		// An item in the Cash Inventory, as Item.nx makes it, with its date and who sent it (FUN_004b0764). A pet
		// has no line for its life.
		void set_cash_item(int32_t itemid, int64_t expiration, const std::string &gift_from);

	private:
		// Lays the tooltip out for `itemid` with these lines under the name, and `desc` if not Item.nx's.
		void build(const std::string &expiry, const std::string &gift, const std::string &desc = {});

		int32_t itemid;
		int64_t expiration;
		int16_t flags;
		bool owned;
		int32_t remain_life;
		std::string owner;
		// Laid out by set_commodity or set_cash_item, so set_item lays it out again.
		bool cash_shop;
		Point<int16_t> size;

		TooltipBox box;
		TooltipIcon icon;

		Texture itemicon;
		Text name;
		// The orange lines under the name.
		std::array<Text, 2> properties;
		int16_t properties_height;
		// A couple's token names the partner under them, in the same orange.
		Text marriage_line;
		int16_t marriage_height;
		Texture expiry_dot;
		Text expiry;
		int16_t expiry_height;
		// "Specially made by" and who, centred under the expiry line.
		Text maker_label;
		Text maker;
		Text gift;
		Text desc;
	};
}
