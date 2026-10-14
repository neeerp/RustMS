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
#include "ItemTooltip.h"

#include "../Data/ItemData.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// Measured from period footage.
		constexpr int16_t WIDTH = 290;
		constexpr int16_t QUAL_Y = 25;
		const Point<int16_t> DESC_POS = { 91, 29 };
		constexpr int16_t DESC_WIDTH = 186;
	}

	ItemTooltip::ItemTooltip()
	{
		itemid = 0;
	}

	bool ItemTooltip::set_item(int32_t iid)
	{
		if (itemid == iid)
			return false;

		itemid = iid;

		if (itemid == 0)
			return false;

		const ItemData &idata = ItemData::get(itemid);

		bool untradable = idata.is_untradable();
		bool unique = idata.is_unique();

		std::string quality;

		if (unique && untradable)
			quality = "One-of-a-kind Item, Untradable";
		else if (unique)
			quality = "One-of-a-kind Item";
		else if (untradable)
			quality = "Untradable";

		itemicon = idata.get_icon(false);
		name = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE, idata.get_name(), WIDTH - 2 * tooltip::NAME_POS.x());
		qual = quality.empty() ? Text() : Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::ORANGE, quality);
		desc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, idata.get_desc(), DESC_WIDTH, true, tooltip::LINE_ADJ);

		int16_t top = qual.empty() ? 0 : qual.height();
		int16_t bottom = std::max<int16_t>(tooltip::ICON_POS.y() + tooltip::ICON_BOX, DESC_POS.y() + desc.height());
		size = Point<int16_t>(WIDTH, top + bottom + tooltip::BOTTOM);

		return true;
	}

	void ItemTooltip::draw(Point<int16_t> pos) const
	{
		if (itemid == 0)
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		icon.draw_dot(pos + tooltip::DOT_POS);
		name.draw(pos + tooltip::NAME_POS);

		if (!qual.empty())
		{
			qual.draw(pos + Point<int16_t>(WIDTH / 2, QUAL_Y));
			pos.shift_y(qual.height());
		}

		icon.draw(itemicon, pos + tooltip::ICON_POS);
		desc.draw(pos + DESC_POS);
	}
}
