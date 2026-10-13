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

#include "../UIDragElement.h"

#include "../Components/AreaButton.h"

#include "../../Graphics/Geometry.h"

namespace ms
{
	class UIWorldMap : public UIDragElement<PosMAP>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::WORLDMAP;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIWorldMap();
		void show_location(int32_t target_map);

		void draw(float inter) const override;

		void update() override;

		void toggle_active() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursor_pos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		static constexpr uint8_t MAPSPOT_TYPE_MAX = 4u;

		void update_world(std::string parent_map);

		enum Buttons
		{
			BT_CLOSE,
			BT_LINK0,
			BT_LINK1,
			BT_LINK2,
			BT_LINK3,
			BT_LINK4,
			BT_LINK5,
			BT_LINK6,
			BT_LINK7,
			BT_LINK8,
			BT_LINK9
		};

		struct MapSpot
		{
			std::string description;
			Texture path;
			std::string title;
			uint8_t type;
			Texture marker;
			std::vector<int32_t> map_ids;
		};

		bool show_path_img;

		int32_t mapid = -1;
		int32_t target_map = -1;

		std::string parent_map;
		std::string user_map;

		Texture border[8];
		Texture title;
		ColorBox fill;
		ColorBox map_edge[2];

		Texture base_img;
		Texture path_img;

		Animation cur_pos;
		Animation npc_pos[MAPSPOT_TYPE_MAX];

		std::map<uint16_t, Texture> link_images;
		std::map<uint16_t, std::string> link_maps;

		std::vector<std::pair<Point<int16_t>, MapSpot>> map_spots;

		Point<int16_t> base_position;
	};
}
