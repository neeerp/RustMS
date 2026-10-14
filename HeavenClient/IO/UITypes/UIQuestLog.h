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
#include <set>

#include "../UIDragElement.h"

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"

#include "../../Character/QuestLog.h"
#include "../../Character/QuestObjectives.h"
#include "../../Graphics/Animation.h"

namespace ms
{
	// The v83 quest log (UIClassic.nx, UIWindow.img/Quest): the list of
	// quests by area, and beside it the selected quest's details.
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

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		Cursor::State send_cursor(bool clicking, Point<int16_t> cursorpos) override;

		// Show a quest in progress, opening it on the In Progress tab.
		void show_quest(int16_t id);

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			TAB0,
			TAB1,
			TAB2,
			CLOSE,
			HIDE,
			FIND_NPC,
			HELPER,
			FORFEIT
		};

		enum Icon : uint8_t
		{
			AVAILABLE,
			IN_PROGRESS,
			READY,
			COMPLETE
		};

		// An area's bar (`quest` < 0) or a quest under it.
		struct Row
		{
			int16_t area;
			int16_t quest;
			Text label;
			// The selected quest's label, white and cut short with "..".
			Text chosen;
			Icon icon;
		};

		// An item icon in the description, in the text's n-th box.
		struct InlineIcon
		{
			Texture texture;
			size_t box;
		};

		void change_tab(uint16_t tabid);
		void refresh_list();
		void select_quest(int16_t id);
		void write_details();
		void show_details(bool open);
		void close();
		void find_npc();
		std::vector<QuestObjective> objectives(int16_t id) const;
		Icon progress_icon(int16_t id) const;
		void scroll_list(int16_t offset);
		void scroll_details(int16_t offset);

		const QuestLog &questlog;
		uint16_t tab = TAB0;
		ClassicTabs tabs;
		Texture list_background, detail_background;
		Texture notices[3];
		Texture fold_boxes[2];
		Texture available_icon, complete_icon;
		Animation progress_animation, ready_animation;
		Slider list_slider, detail_slider;
		std::vector<Row> rows;
		std::set<int16_t> folded[3];
		Animation portrait;
		Text title, jobs, levels, description;
		std::vector<InlineIcon> icons;
		std::vector<QuestObjective> shown_goals;
		bool details_open = false;
		int16_t selected = -1, list_offset = 0, detail_offset = 0;
		int32_t target_npc = 0, target_map = -1;
		Point<int16_t> cursor_relative;
		uint64_t seen_revision = 0;
		int16_t seen_job = -1, seen_level = -1;
	};
}
