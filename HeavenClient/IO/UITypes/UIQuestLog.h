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
#include "../../Util/NpcTextMarkup.h"

namespace ms
{
	// A quest's text with the quest-only placeholders filled in, which
	// NpcTextMarkup leaves: #c how many of an item the player holds, #a a
	// kill count and #R a quest's record. Line breaks become the renderer's
	// escapes.
	std::string quest_text(const std::string& text, int16_t quest_id);

	// The images in a quest's text, each in a box of the text: item icons
	// (#i, #v) and images by path (a party quest's rank, PQ Mania's gauge).
	class QuestTextImages
	{
	public:
		// Take the images of `tags` and give `description` their boxes.
		void set(const std::vector<NpcTextMarkup::InlineImage>& tags, Text& description);
		// Draw those wholly inside `clip`, the text drawn at `text`.
		void draw(const Text& description, Point<int16_t> text, Range<int16_t> clip) const;

	private:
		struct Image
		{
			Texture texture;
			// The box's size, and where in it the texture's origin goes.
			Point<int16_t> size;
			Point<int16_t> anchor;
			// A gauge's slices (#jgaugePqS#), or -1 for a plain image.
			int32_t slices = -1;
		};

		std::vector<Image> images;
		Texture gauge_slice;
	};

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
		void doubleclick(Point<int16_t> cursorpos) override;

		// Open tab `tab` (0 Available, 1 In Progress, 2 Completed) and select
		// quest `id` there, unfolding its area, if listed (FUN_00882b2e).
		void show_quest(int16_t id, uint16_t tab = 1);

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

		void change_tab(uint16_t tabid);
		void refresh_list();
		void select_quest(int16_t id);
		void write_details();
		void show_details(bool open);
		void close();
		void find_npc();
		void give_up(int16_t id);
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
		QuestTextImages images;
		// The NPC and map names in `description`.
		std::vector<NpcTextMarkup::Link> links;
		std::vector<QuestObjective> shown_goals;
		bool details_open = false;
		int16_t selected = -1, list_offset = 0, detail_offset = 0;
		// The NPC FIND NPC and the portrait find: the quest's, or the last
		// one double-clicked in its text.
		int32_t start_npc = 0;
		Point<int16_t> cursor_relative;
		uint64_t seen_revision = 0;
		int16_t seen_job = -1, seen_level = -1;
	};
}
