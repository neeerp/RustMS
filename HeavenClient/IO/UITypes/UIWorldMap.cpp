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
#include "UIWorldMap.h"

#include "../UI.h"

#include "../Gameplay/Stage.h"
#include "../Util/Misc.h"

#include "../IO/Components/MapleButton.h"

#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>
#include <algorithm>

namespace ms
{
	namespace
	{
		// Laid out from footage, against the window's top-left corner. The
		// frame fits a 640x470 world map with a margin of 14 at the sides.
		constexpr int16_t WIDTH = 668, HEIGHT = 524;
		constexpr int16_t EDGE = 7, TOP = 33, BOTTOM = 18;
		const Point<int16_t> TITLE = { 9, 10 };
		const Point<int16_t> CLOSE = { WIDTH - 21, 6 };
		const Point<int16_t> MAP = { 14, 35 };
		constexpr int16_t MAP_WIDTH = 640, MAP_HEIGHT = 470;

		std::string world_for_map(int32_t field)
		{
			// Map IDs are not world-map filenames. Prefer the most detailed
			// asset whose MapList actually contains this field.
			static const auto worlds = [] {
				std::map<int32_t, std::pair<size_t, std::string>> result;
				for (auto world : nl::nx::map["WorldMap"])
				{
					std::vector<int32_t> fields;
					for (auto spot : world["MapList"])
						for (auto id : spot["mapNo"]) fields.push_back(id.get_integer());
					std::string name = world.name();
					if (name.size() < 4) continue;
					name.resize(name.size() - 4);
					for (auto id : fields)
					{
						auto found = result.find(id);
						if (found == result.end() || fields.size() < found->second.first)
							result[id] = {fields.size(), name};
					}
				}
				return result;
			}();
			auto found = worlds.find(field);
			return found == worlds.end() ? "WorldMap" : found->second.second;
		}
	}

	UIWorldMap::UIWorldMap() : UIDragElement<PosMAP>(Point<int16_t>(WIDTH, 20))
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["WorldMap"];
		nl::node MapHelper = nl::nx::map["MapHelper.img"]["worldMap"];

		for (size_t i = 0; i < 8; i++)
			border[i] = src["Border"][std::to_string(i)];

		title = src["title"];
		fill = ColorBox(WIDTH - 2 * EDGE, HEIGHT - TOP - BOTTOM, Color::Name::WHITE, 1.0f);

		// A grey line along the map's top and left edges.
		map_edge[0] = ColorBox(MAP_WIDTH + 1, 1, Color::Name::DUSTYGRAY, 1.0f);
		map_edge[1] = ColorBox(1, MAP_HEIGHT, Color::Name::DUSTYGRAY, 1.0f);

		cur_pos = MapHelper["curPos"];

		for (size_t i = 0; i < MAPSPOT_TYPE_MAX; i++)
			npc_pos[i] = MapHelper["npcPos" + std::to_string(i)];

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);

		dimension = Point<int16_t>(WIDTH, HEIGHT);
	}

	void UIWorldMap::draw(float alpha) const
	{
		int16_t right = WIDTH - EDGE;
		int16_t bottom = HEIGHT - BOTTOM;
		int16_t top_right = border[2].get_dimensions().y();

		border[0].draw(position);
		border[1].draw(DrawArgument(position + Point<int16_t>(EDGE, 0), Point<int16_t>(right - EDGE, 0)));
		border[2].draw(position + Point<int16_t>(right, 0));
		border[3].draw(DrawArgument(position + Point<int16_t>(0, TOP), Point<int16_t>(0, bottom - TOP)));
		border[4].draw(DrawArgument(position + Point<int16_t>(right, top_right), Point<int16_t>(0, bottom - top_right)));
		border[5].draw(position + Point<int16_t>(0, bottom));
		border[6].draw(DrawArgument(position + Point<int16_t>(EDGE, bottom), Point<int16_t>(right - EDGE, 0)));
		border[7].draw(position + Point<int16_t>(right, bottom));
		fill.draw(DrawArgument(position + Point<int16_t>(EDGE, TOP)));
		title.draw(position + TITLE);

		map_edge[0].draw(DrawArgument(position + MAP - Point<int16_t>(1, 1)));
		map_edge[1].draw(DrawArgument(position + MAP - Point<int16_t>(1, 0)));
		base_img.draw(position + base_position);

		if (link_images.size() > 0)
		{
			for (auto &iter : buttons)
			{
				if (const auto button = iter.second.get())
				{
					if (iter.first >= Buttons::BT_LINK0 && button->get_state() == Button::State::MOUSEOVER)
					{
						if (link_images.find(iter.first) != link_images.end())
						{
							link_images.at(iter.first).draw(position + base_position);
							break;
						}
					}
				}
			}
		}

		if (show_path_img)
			path_img.draw(position + base_position);

		for (auto spot : map_spots)
		{
			spot.second.marker.draw(spot.first + position + base_position);
			if (std::find(spot.second.map_ids.begin(), spot.second.map_ids.end(), target_map) != spot.second.map_ids.end())
				npc_pos[std::min<uint8_t>(spot.second.type, MAPSPOT_TYPE_MAX - 1)].draw(spot.first + position + base_position, alpha);
		}

		for (auto spot : map_spots)
		{
			if (std::find(spot.second.map_ids.begin(), spot.second.map_ids.end(), mapid) != spot.second.map_ids.end())
			{
				npc_pos[std::min<uint8_t>(spot.second.type, MAPSPOT_TYPE_MAX - 1)].draw(spot.first + position + base_position, alpha);
				cur_pos.draw(spot.first + position + base_position, alpha);
				break;
			}
		}

		UIElement::draw_buttons(alpha);
	}

	void UIWorldMap::show_location(int32_t target)
	{
		makeactive();
		mapid = Stage::get().get_mapid();
		target_map = target;
		user_map = world_for_map(mapid);
		update_world(world_for_map(target));
	}

	void UIWorldMap::update()
	{
		int32_t mid = Stage::get().get_mapid();

		if (mid != mapid)
		{
			mapid = mid;
			auto parent_map = world_for_map(mapid);
			user_map = parent_map;

			update_world(parent_map);
		}

		for (size_t i = 0; i < MAPSPOT_TYPE_MAX; i++)
			npc_pos[i].update(1);

		cur_pos.update();

		UIElement::update();
	}

	void UIWorldMap::toggle_active()
	{
		UIElement::toggle_active();

		if (!active)
			update_world(user_map);
	}

	void UIWorldMap::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			if (parent_map == "")
			{
				toggle_active();

				update_world(user_map);
			} else
			{
				Sound(Sound::Name::SELECTMAP).play();

				update_world(parent_map);
			}
		}
	}

	UIElement::Type UIWorldMap::get_type() const
	{
		return TYPE;
	}

	Button::State UIWorldMap::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_CLOSE:
				deactivate();
				break;
			default:
				break;
		}

		if (buttonid >= Buttons::BT_LINK0)
		{
			update_world(link_maps[buttonid]);

			return Button::State::IDENTITY;
		}

		return Button::State::NORMAL;
	}

	void UIWorldMap::remove_cursor()
	{
		UIDragElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::WORLDMAP);

		show_path_img = false;
	}

	Cursor::State UIWorldMap::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		show_path_img = false;
		bool on_spot = false;

		for (auto path : map_spots)
		{
			Point<int16_t> p = path.first + position + base_position - 10;
			Point<int16_t> d = p + path.second.marker.get_dimensions();
			Rectangle<int16_t> abs_bounds = Rectangle<int16_t>(p, d);

			if (abs_bounds.contains(cursorpos))
			{
				path_img = path.second.path;
				show_path_img = path_img.is_valid();
				on_spot = true;

				UI::get().show_map(Tooltip::Parent::WORLDMAP, path.second.title, path.second.description);
				break;
			}
		}

		if (!on_spot)
			UI::get().clear_tooltip(Tooltip::Parent::WORLDMAP);

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIWorldMap::update_world(std::string map)
	{
		nl::node WorldMap = nl::nx::map["WorldMap"][map + ".img"];

		if (!WorldMap)
			WorldMap = nl::nx::map["WorldMap"]["WorldMap.img"];

		base_img = WorldMap["BaseImg"][0];
		base_position = MAP + base_img.get_origin();
		parent_map = std::string(WorldMap["info"]["parentMap"]);

		link_images.clear();
		link_maps.clear();

		for (auto &iter : buttons)
			if (const auto button = iter.second.get())
				if (iter.first >= Buttons::BT_LINK0)
					button->set_active(false);

		size_t i = Buttons::BT_LINK0;

		for (nl::node link : WorldMap["MapLink"])
		{
			nl::node l = link["link"];
			Texture link_image = l["linkImg"];

			link_images[i] = link_image;
			link_maps[i] = std::string(l["linkMap"]);

			buttons[i] = std::make_unique<AreaButton>(base_position - link_image.get_origin(),
													  link_image.get_dimensions());
			buttons[i]->set_active(true);

			i++;
		}

		nl::node mapImage = nl::nx::map["MapHelper.img"]["worldMap"]["mapImage"];

		map_spots.clear();

		for (nl::node list : WorldMap["MapList"])
		{
			nl::node desc = list["desc"];
			nl::node mapNo = list["mapNo"];
			nl::node path = list["path"];
			nl::node spot = list["spot"];
			nl::node title = list["title"];
			nl::node type = list["type"];
			nl::node marker = mapImage[type];

			std::vector<int32_t> map_ids;

			for (nl::node map_no : mapNo)
				map_ids.push_back(map_no);

			if (!desc && !title)
			{
				NxHelper::Map::MapInfo map_info = NxHelper::Map::get_map_info_by_id(mapNo[0]);

				map_spots.emplace_back(std::make_pair<Point<int16_t>, MapSpot>(spot, {map_info.description, path,
																					  map_info.full_name, type, marker,
																					  map_ids}));
			} else
			{
				map_spots.emplace_back(std::make_pair<Point<int16_t>, MapSpot>(spot,
																			   {desc, path, title, type, marker, map_ids}));
			}
		}
	}
}
