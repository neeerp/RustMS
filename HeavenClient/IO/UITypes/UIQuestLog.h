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

#include <cstdint>

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/QuestLog.h"

namespace ms
{
	class UIQuestLog : public UIDragElement<PosQUEST>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::QUESTLOG;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIQuestLog(const QuestLog &questLog);

		void draw(float inter) const override;
		void update() override;
		void toggle_active() override;
		void update_screen(int16_t width, int16_t height) override;
		void send_scroll(double yoffset) override;
		void doubleclick(Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		Cursor::State send_cursor(bool clicking, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void change_tab(uint16_t tabid);
		void refresh_list();
		void select_quest(int16_t id);
		void close();
		void find_npc();
		int32_t local_npc_oid() const;

		enum Buttons : uint16_t
		{
			TAB0,
			TAB1,
			TAB2,
			CLOSE,
			SEARCH,
			ALL_LEVEL,
			MY_LOCATION,
			DETAIL_CLOSE,
			ACCEPT,
			FINISH,
			FIND_NPC,
			HELPER,
			FORFEIT,
			MY_LEVEL,
			ALL_LOCATION,
			RECOMMENDED
		};

		const QuestLog &questlog;
		uint16_t tab;
		std::vector<Sprite> notice_sprites;
		Textfield search;
		Text placeholder;
		Slider slider;
		Slider detail_slider;
		Texture search_area;
		Texture complete_count;
		Texture info_background, info_paper, info_header, info_tip;
		Texture quest_icons[3];
		Texture reward_background, reward_heading, basic_reward;
		Animation portrait;
		Text heading, level_text, description, objective, location, reward, completed_on;
		Text completion_count, empty_detail;
		struct QuestRow { int16_t id; Text title; bool eligible; };
		std::vector<QuestRow> rows;
		Text group_title;
		bool collapsed = false, all_levels = false, current_location = false;
		bool click_down = false, detail_open = true;
		int16_t selected = -1, list_offset = 0, detail_offset = 0, reward_y = 245, detail_height = 235;
		int32_t target_npc = 0, target_map = -1;
		Point<int16_t> cursor_relative;
		uint64_t seen_revision = 0;
		int16_t seen_job = -1, seen_level = -1;
		int32_t seen_map = -1;
		std::string seen_search;
		static constexpr int16_t DETAIL_X = 295, ROW_Y = 82, ROW_HEIGHT = 18, VISIBLE_ROWS = 15;
	};
}
