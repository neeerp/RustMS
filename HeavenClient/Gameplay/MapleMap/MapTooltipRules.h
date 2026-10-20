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

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <map>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// v83's map tooltips: a map's ToolTip zones, whose String
	// ToolTipHelp.img/Mapobject text shows while the cursor is over one or
	// the player stands in its "<N>char" zone, and the minimap's portal tips.
	namespace map_tooltips
	{
		// A ToolTip zone's x1, y1, x2 and y2, in map coordinates.
		struct Zone
		{
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;
		};

		// What a ToolTip child's name makes it: "<N>char" is the zone the
		// player stands in to see zone N's text, anything else zone N.
		struct ZoneName
		{
			bool character;
			int32_t key;
		};

		// ZoneSet::Load (0x8f622c) takes a name ending in "char" as a
		// character zone keyed by atoi of the rest.
		inline ZoneName parse_zone_name(const std::string &name)
		{
			const std::string suffix = "char";
			size_t length = name.size();

			if (length >= suffix.size() && name.compare(length - suffix.size(), suffix.size(), suffix) == 0)
				return { true, std::atoi(name.substr(0, length - suffix.size()).c_str()) };

			return { false, std::atoi(name.c_str()) };
		}

		// PtInRect: the left and top edges are in, the right and bottom out.
		inline bool contains(Zone zone, int32_t x, int32_t y)
		{
			return x >= zone.left && x < zone.right && y >= zone.top && y < zone.bottom;
		}

		// The first zone the cursor is over, or -1 (HitTest 0x8f6c44).
		inline int32_t zone_at(const std::map<int32_t, Zone> &zones, int32_t x, int32_t y)
		{
			for (const auto &[key, zone] : zones)
				if (contains(zone, x, y))
					return key;

			return -1;
		}

		// The character zones the player stands in (0x8f80e6).
		inline std::vector<int32_t> char_zones_at(const std::map<int32_t, Zone> &zones, int32_t x, int32_t y)
		{
			std::vector<int32_t> keys;

			for (const auto &[key, zone] : zones)
				if (contains(zone, x, y))
					keys.push_back(key);

			return keys;
		}

		// A character zone's box hangs from its zone's bottom right
		// (GetCharTip 0x8f6d34).
		inline std::pair<int32_t, int32_t> anchor(Zone zone)
		{
			return { zone.right, zone.bottom };
		}

		// v83 draws these tips in Arial 12 (FUN_008e49b5): a line's height
		// (I: the font's size), how far below the text's top its baseline
		// sits (I: Arial's ascent), and how far apart wrapped lines are (I:
		// as in the item tooltips).
		constexpr int16_t FONT_HEIGHT = 12;
		constexpr int16_t BASELINE = 11;
		constexpr int16_t LINE = 16;

		// A box's size and where its text goes in it.
		struct Box
		{
			int16_t width;
			int16_t height;
			int16_t title_x;
			int16_t title_y;
			int16_t desc_x;
			int16_t desc_y;
			int16_t wrap;
		};

		// The compact box SetToolTip_TitleDesc (0x8e7150) makes for the map
		// tips, for a title as wide as given (0 without one) and a
		// description as tall (0 without one). Only the title widens it,
		// and the description wraps inside that. Without a title it has no
		// width, and so shows nothing.
		inline Box compact_box(int16_t title_width, int16_t desc_height)
		{
			constexpr int16_t MARGIN = 6;
			Box box = {};
			box.height = 2 * MARGIN - 6;
			box.desc_x = 10;
			box.desc_y = MARGIN;

			if (title_width > 0)
			{
				box.width = title_width + 14;
				box.height += FONT_HEIGHT + 8;
				box.title_x = (box.width - title_width) / 2;
				box.title_y = MARGIN - 2;
				box.desc_y += FONT_HEIGHT + 8;
			}

			box.wrap = box.width - 15;
			box.height += desc_height > 0 ? desc_height : -4;

			return box;
		}

		// The normal box SetToolTip_TitleDesc (0x8e7150) makes for a button's
		// help and the Stat window's zones: 270 wide unless the title needs
		// more, the description wrapped at 255 below the title, or in its
		// place without one.
		inline Box help_box(int16_t title_width, int16_t desc_height)
		{
			constexpr int16_t MARGIN = 10;
			constexpr int16_t WIDTH = 270;
			Box box = {};
			box.width = WIDTH;
			box.height = 2 * MARGIN - 6;
			box.desc_x = 10;
			box.desc_y = MARGIN;
			box.wrap = WIDTH - 15;

			if (title_width > 0)
			{
				box.width = std::max<int16_t>(WIDTH, title_width + 16);
				box.height += FONT_HEIGHT + 8;
				box.title_x = (box.width - title_width) / 2;
				box.title_y = MARGIN - 2;
				box.desc_y += FONT_HEIGHT + 8;
			}

			box.height += desc_height > 0 ? desc_height : -4;

			return box;
		}

		// A Stat window zone from ToolTipHelp.img/Game/UIWnd, against its
		// window's top left.
		struct HelpZone
		{
			Zone zone;
			std::string title;
			std::string desc;
		};

		// The first of the first limit zones (all for 0) the point is in, or
		// -1 (HitTest 0x8f6c44).
		inline int32_t help_at(const std::vector<HelpZone> &zones, int32_t x, int32_t y, size_t limit = 0)
		{
			size_t count = limit > 0 ? std::min(limit, zones.size()) : zones.size();

			for (size_t i = 0; i < count; i++)
				if (contains(zones[i].zone, x, y))
					return static_cast<int32_t>(i);

			return -1;
		}

		// How many of the Stat window's zones have help (0x8c52d7): a
		// beginner, Noblesse, Legend or Evan of level 10 or less only gets
		// Name to Fame, the first 8.
		inline size_t stat_help_limit(int32_t level, int32_t job)
		{
			bool beginner = job == 0 || job == 1000 || job == 2000 || job == 2001;

			return level <= 10 && beginner ? 8 : 0;
		}

		// Whether a point in the Stat window is on its EXP line, which shows
		// the EXP and what the level needs in the one-line box instead.
		inline bool on_stat_exp(int32_t x, int32_t y)
		{
			return x > 61 && x < 172 && y > 140 && y < 155;
		}

		// The one-line box (0x8e6e7d) for a portal's tip on the minimap.
		inline Box line_box(int16_t text_width)
		{
			return { static_cast<int16_t>(text_width + 8), FONT_HEIGHT + 6, 4, 1, 0, 0, 0 };
		}

		// Where a box on the screen goes (0x8f3141): at the given spot, but
		// not left of or above the screen, then not past its last pixel.
		inline std::pair<int16_t, int16_t> clamp_screen(int16_t x, int16_t y, int16_t width, int16_t height,
														int16_t screen_width, int16_t screen_height)
		{
			x = x < 0 ? 0 : x;
			y = y < 0 ? 0 : y;

			if (x + width > screen_width - 1)
				x = screen_width - 1 - width;

			if (y + height > screen_height - 1)
				y = screen_height - 1 - height;

			return { x, y };
		}

		// The spot of a portal's marker the cursor must be in for its tip,
		// around where the marker stands on the minimap (0x85b789).
		inline Zone minimap_portal_spot(int32_t x, int32_t y)
		{
			return { x - 2, y - 5, x + 3, y + 2 };
		}

		// Whether the minimap shows a portal's marker: the visible portals,
		// types 2 and 7 (0x85a172).
		inline bool minimap_shows(int32_t type)
		{
			return type == 2 || type == 7;
		}

		// Whether a portal on the minimap has a tip: one shown that leads
		// somewhere and doesn't hide it (0x85b759).
		inline bool minimap_tip(int32_t type, int32_t target, bool hidden)
		{
			return minimap_shows(type) && target != -1 && !hidden;
		}
	}
}
