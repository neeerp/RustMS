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

#include "../Constants.h"

#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	MapTooltip::MapTooltip() : fillwidth(map_tooltip::MIN_WIDTH), fillheight(0)
	{
		nl::node Frame = nl::nx::ui["UIToolTip.img"]["Item"]["Frame2"];
		nl::node WorldMap = nl::nx::ui["UIWindow2.img"]["ToolTip"]["WorldMap"];

		frame = Frame;
		cover = Frame["cover"];
		Mob = WorldMap["Mob"];
		Npc = WorldMap["Npc"];
		Party = WorldMap["Party"];
	}

	void MapTooltip::draw(Point<int16_t> pos) const
	{
		if (name_label.empty())
			return;

		int16_t max_width = Constants::Constants::get().get_viewwidth();
		int16_t max_height = Constants::Constants::get().get_viewheight();

		if (parent == Tooltip::Parent::MINIMAP && mob_labels.empty() && npc_labels.empty())
		{
			if (desc_label.empty())
			{
				int16_t new_width = name_simple.width();
				int16_t new_height = name_simple.height();

				int16_t cur_width = pos.x() + new_width + 21;
				int16_t cur_height = pos.y() + new_height + 40;

				int16_t adj_x = cur_width - max_width;
				int16_t adj_y = cur_height - max_height;

				if (adj_x > 0)
					pos.shift_x(adj_x * -1);

				if (adj_y > 0)
					pos.shift_y(adj_y * -1);

				frame.draw(pos + Point<int16_t>(new_width / 2 + 2, new_height - 7), new_width - 14, new_height - 18);

				if (new_height > 18)
					name_simple.draw(pos);
				else
					name_simple.draw(pos + Point<int16_t>(1, -3));
			} else
			{
				int16_t name_width = name_label.width();
				int16_t name_height = name_label.height();

				int16_t desc_width = desc_simple.width();

				int16_t new_width = (name_width > desc_width) ? name_width : desc_width;
				int16_t new_height = name_height + desc_simple.height() - 11;

				int16_t cur_width = pos.x() + new_width + 21;
				int16_t cur_height = pos.y() + new_height + 40;

				int16_t adj_x = cur_width - max_width;
				int16_t adj_y = cur_height - max_height;

				if (adj_x > 0)
					pos.shift_x(adj_x * -1);

				if (adj_y > 0)
					pos.shift_y(adj_y * -1);

				int16_t half_width = new_width / 2;

				frame.draw(pos + Point<int16_t>(half_width + 2, new_height - 7 + BOTTOM_PADDING), new_width - 14,
						   new_height - 18 + BOTTOM_PADDING);
				cover.draw(pos + Point<int16_t>(-5, -2));
				name_label.draw(pos + Point<int16_t>(half_width, -2));

				pos.shift_y(name_height);

				separator.draw(pos + SEPARATOR_ADJ);
				desc_simple.draw(pos + Point<int16_t>(half_width, -3));
			}
		} else
		{
			int16_t cur_width = pos.x() + fillwidth + 21;
			int16_t cur_height = pos.y() + fillheight + 40;

			int16_t adj_x = cur_width - max_width;
			int16_t adj_y = cur_height - max_height;

			if (adj_x > 0)
				pos.shift_x(adj_x * -1);

			if (adj_y > 0)
				pos.shift_y(adj_y * -1);

			int16_t half_width = fillwidth / 2;

			frame.draw(pos + Point<int16_t>(half_width + 2, fillheight - 7 + BOTTOM_PADDING), fillwidth - 14,
					   fillheight - 18 + BOTTOM_PADDING);
			cover.draw(pos + Point<int16_t>(-5, -2));
			name_label.draw(pos + Point<int16_t>(half_width, 0));

			int16_t name_height = name_label.height();

			if (!desc_label.empty())
			{
				pos.shift_y(name_height + 4);

				desc_label.draw(pos + Point<int16_t>(4, 0));

				pos.shift_y(desc_label.height() + BOTTOM_PADDING);
			} else
			{
				pos.shift_y(name_height + BOTTOM_PADDING);
			}

			if (!mob_labels.empty())
			{
				separator.draw(pos + SEPARATOR_ADJ);
				Mob.draw(pos + LIFE_ICON_ADJ);

				for (const Text& label : mob_labels)
				{
					label.draw(pos + LIFE_LABEL_ADJ);
					pos.shift_y(label.height());
				}

				pos.shift_y(8);
			}

			if (!npc_labels.empty())
			{
				separator.draw(pos + SEPARATOR_ADJ);
				Npc.draw(pos + LIFE_ICON_ADJ);

				for (const Text& label : npc_labels)
				{
					label.draw(pos + LIFE_LABEL_ADJ);
					pos.shift_y(label.height());
				}
			}
		}
	}

	void MapTooltip::set_map(Tooltip::Parent p, std::string n, std::string d, int32_t m, bool b)
	{
		if (p == parent && n == name && d == description && m == mapid && b == bolded)
			return;

		parent = p;
		name = n;
		description = d;
		mapid = m;
		bolded = b;

		name_label = Text();
		name_simple = Text();
		desc_label = Text();
		desc_simple = Text();
		mob_labels.clear();
		npc_labels.clear();
		fillwidth = map_tooltip::MIN_WIDTH;
		fillheight = 0;

		if (name.empty() || (parent != Tooltip::Parent::WORLDMAP && parent != Tooltip::Parent::MINIMAP))
			return;

		Text::Font font = bolded ? Text::Font::A12B : Text::Font::A12M;
		name_label = Text(font, Text::Alignment::CENTER, Color::Name::WHITE, name);
		name_simple = Text(font, Text::Alignment::LEFT, Color::Name::WHITE, name);

		int16_t wrap = map_tooltip::wrap_width(name_label.width());
		int16_t desc_height = 0;

		if (!description.empty())
		{
			desc_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, description, wrap);
			desc_simple = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE, description, wrap);
			desc_height = desc_label.height();
		}

		map_tooltip::Life life = map_tooltip::pick_life(NxHelper::Map::get_life_on_map(mapid));

		for (const std::string& mob : life.mobs)
			mob_labels.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::CHARTREUSE, mob);

		for (const std::string& npc : life.npcs)
			npc_labels.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::MALIBU, npc);

		int16_t line_height = !mob_labels.empty() ? mob_labels[0].height()
							: !npc_labels.empty() ? npc_labels[0].height() : 0;
		map_tooltip::Size size = map_tooltip::size(name_label.width(), name_label.height(), desc_height, life,
												   line_height);
		fillwidth = size.width;
		fillheight = size.height;

		// The minimap's frame hugs its text.
		int16_t separator_width = fillwidth;

		if (parent == Tooltip::Parent::MINIMAP && !description.empty())
			separator_width = std::max(name_label.width(), desc_simple.width());

		separator = ColorLine(separator_width - 6, Color::Name::WHITE, 0.40f);
	}

	void MapTooltip::reset()
	{
		set_map(Tooltip::Parent::NONE, "", "", 0, false);
	}
}
