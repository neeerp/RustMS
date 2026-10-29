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
#include <vector>

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

		// Opens on the player's field, as the world map key and the
		// minimap's button do, or says the world map isn't available there.
		static void open();
		// Opens with the NPC's fields marked, as FIND NPC does, or says no
		// world map shows them (v83 FUN_0087e769).
		static void find_npc(int32_t npc);
		// Opens on the field as if the player stood there, as a map's name
		// in the quest log does, or says no world map shows it (v83
		// FUN_0087e809).
		static void find_map(int32_t map);

		void show_npc(const std::vector<int32_t>& maps, const std::string& world);
		void show_map(int32_t map, const std::string& world);

		void draw(float inter) const override;

		void update() override;

		void toggle_active() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursor_pos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		// Back to the parent world map, as Esc does (v83 FUN_009ee507).
		void rightclick(Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		// A map link plays only SelectMap (v83 FUN_009ee30e).
		bool has_own_click_sound(uint16_t buttonid) const override;

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

		// The field marked as the player's, and the one the player is on.
		int32_t mapid = -1;
		int32_t seen_map = -1;
		std::vector<int32_t> target_maps;

		std::string parent_map;
		std::string user_map;

		Texture border[8];
		Texture title;
		ColorBox fill;
		ColorBox map_edge[2];

		Texture base_img;
		Texture path_img;

		Animation cur_pos;
		Animation love_pos;
		Animation npc_pos[MAPSPOT_TYPE_MAX];

		std::map<uint16_t, Texture> link_images;
		std::map<uint16_t, std::string> link_maps;

		std::vector<std::pair<Point<int16_t>, MapSpot>> map_spots;

		Point<int16_t> base_position;
	};
}
