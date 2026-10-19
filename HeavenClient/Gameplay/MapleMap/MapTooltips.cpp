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
#include "MapTooltips.h"

#include "../../IO/UI.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <vector>

namespace ms
{
	namespace
	{
		// v83 takes only string values, anything else as "" (0x8f69ee).
		std::string text_of(nl::node src)
		{
			return src.data_type() == nl::node::type::string ? src.get_string() : "";
		}
	}

	MapTooltips::MapTooltips(int32_t mapid, nl::node src)
	{
		// The first zone of a name wins; NX keeps no duplicates anyway.
		for (nl::node zone : src["ToolTip"])
		{
			map_tooltips::ZoneName name = map_tooltips::parse_zone_name(zone.name());
			auto at = [&zone](const char *key) { return static_cast<int32_t>(zone[key].get_integer()); };
			map_tooltips::Zone rect = { at("x1"), at("y1"), at("x2"), at("y2") };
			(name.character ? char_zones : zones).emplace(name.key, rect);
		}

		// The text comes by the field's own id, never its link. v83 pairs
		// "<N>char" with the Nth numbered zone in WZ order, which NX doesn't
		// keep; this pairs it with zone N, as WZ order does in every map but
		// Ellinia and Ariant.
		nl::node strings = nl::nx::string["ToolTipHelp.img"]["Mapobject"][std::to_string(mapid)];

		for (nl::node text : strings)
		{
			map_tooltips::ZoneName name = map_tooltips::parse_zone_name(text.name());

			if (!name.character && zones.count(name.key))
				texts[name.key] = { text_of(text["Title"]), text_of(text["Desc"]) };
		}
	}

	MapTooltips::MapTooltips() {}

	void MapTooltips::hover(Point<int16_t> cursor)
	{
		int32_t key = map_tooltips::zone_at(zones, cursor.x(), cursor.y());

		if (key != hovered)
		{
			hovered = key;
			balloons.erase(key);
		}

		if (key == -1)
		{
			UI::get().clear_tooltip(Tooltip::Parent::FIELD);

			return;
		}

		// A zone without text leaves the last tip up (0x8e7150).
		auto text = texts.find(key);

		if (text != texts.end())
			UI::get().show_zone(Tooltip::Parent::FIELD, text->second.title, text->second.desc);
	}

	void MapTooltips::leave()
	{
		hovered = -1;
		UI::get().clear_tooltip(Tooltip::Parent::FIELD);
	}

	void MapTooltips::update(Point<int16_t> player)
	{
		if (char_zones.empty())
			return;

		std::vector<int32_t> keys = map_tooltips::char_zones_at(char_zones, player.x(), player.y());

		for (auto iter = balloons.begin(); iter != balloons.end();)
		{
			if (iter->first == hovered || std::find(keys.begin(), keys.end(), iter->first) == keys.end())
				iter = balloons.erase(iter);
			else
				++iter;
		}

		for (int32_t key : keys)
		{
			if (key == hovered || balloons.count(key))
				continue;

			auto zone = zones.find(key);
			auto text = texts.find(key);

			if (zone == zones.end() || text == texts.end())
				continue;

			auto [x, y] = map_tooltips::anchor(zone->second);
			Balloon &balloon = balloons[key];
			balloon.position = Point<int16_t>(x, y);
			balloon.tip.set_zone(text->second.title, text->second.desc);
		}
	}

	void MapTooltips::draw(Point<int16_t> viewpos) const
	{
		for (const auto &[key, balloon] : balloons)
			balloon.tip.draw_box(balloon.position + viewpos);
	}
}
