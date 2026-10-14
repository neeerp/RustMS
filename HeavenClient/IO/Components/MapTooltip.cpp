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
#include "MapTooltip.h"

#include "MapTooltipLayout.h"

namespace ms
{
	namespace
	{
		// Where the text starts in the box; glyphs sit low in their lines.
		const Point<int16_t> TEXT_POS = { map_tooltip::PADDING_X, 2 };
	}

	MapTooltip::MapTooltip() {}

	void MapTooltip::draw(Point<int16_t> pos) const
	{
		if (name_label.empty())
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		pos += TEXT_POS;
		name_label.draw(pos);

		if (!desc_label.empty())
			desc_label.draw(pos + Point<int16_t>(0, name_label.height() + map_tooltip::DESC_GAP));
	}

	void MapTooltip::set_map(Tooltip::Parent p, std::string n, std::string d)
	{
		if (p == parent && n == name && d == description)
			return;

		parent = p;
		name = n;
		description = d;

		name_label = Text();
		desc_label = Text();
		size = {};

		if (name.empty() || (parent != Tooltip::Parent::WORLDMAP && parent != Tooltip::Parent::MINIMAP))
			return;

		name_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, name);

		int16_t desc_width = 0, desc_height = 0;

		if (!description.empty())
		{
			desc_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, description,
							  map_tooltip::wrap_width(name_label.width()));
			desc_width = desc_label.width();
			desc_height = desc_label.height();
		}

		map_tooltip::Size s = map_tooltip::size(name_label.width(), name_label.height(), desc_width, desc_height);
		size = Point<int16_t>(s.width, s.height);
	}

	void MapTooltip::reset()
	{
		set_map(Tooltip::Parent::NONE, "", "");
	}
}
