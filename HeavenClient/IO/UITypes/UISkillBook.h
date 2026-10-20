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

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"
#include "../Components/StatefulIcon.h"

#include "../../Character/CharStats.h"
#include "../../Character/SkillBook.h"

namespace ms
{
	// The v83 skill window (UIClassic.nx UIWindow.img/Skill), laid out from
	// period footage: one tab per job advancement, the book of the selected
	// one, four skill rows, and the skill points left.
	class UISkillBook : public UIDragElement<PosSKILL>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SKILLBOOK;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UISkillBook(const CharStats &stats, const SkillBook &skillbook);

		void draw(float alpha) const override;

		void toggle_active() override;

		void update() override;

		void doubleclick(Point<int16_t> cursorpos) override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void update_stat(MapleStat::Id stat, int16_t value);

		void update_skills(int32_t skill_id);

	protected:
		Button::State button_pressed(uint16_t id) override;

	private:
		class SkillIcon : public StatefulIcon::Type
		{
		public:
			SkillIcon(int32_t skill_id);

			void drop_on_stage() const override {}
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const override;
			Keyboard::Mapping get_mapping() const override;

			void set_count(int16_t) override
			{}

			void set_state(StatefulIcon::State) override
			{}

			Icon::IconType get_type() override;

		private:
			int32_t skill_id;
		};

		class SkillDisplayMeta
		{
		public:
			SkillDisplayMeta(int32_t id, int32_t level);

			void draw(Point<int16_t> position) const;

			int32_t get_id() const;

			int32_t get_level() const;

			StatefulIcon *get_icon() const;

		private:
			int32_t id;
			int32_t level;
			std::unique_ptr<StatefulIcon> icon;
			Text name_text;
			Text level_text;
		};

		void change_job(uint16_t id);

		void change_sp();

		void change_tab(uint16_t new_tab);

		void load_skills();

		void change_offset(uint16_t new_offset);

		void show_skill(int32_t skill_id);

		void clear_tooltip();

		bool can_raise(int32_t skill_id) const;

		void spend_sp(int32_t skill_id);

		Job::Level joblevel_by_tab(uint16_t tab) const;

		uint16_t tab_count() const;

		// The visible row under a position relative to the window, or -1.
		int16_t row_by_position(Point<int16_t> cursorpos) const;

		void close();

		// Closes the macro window, which belongs to this one.
		void close_macros();

		bool check_required(int32_t id) const;

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_TAB0,
			BT_TAB1,
			BT_TAB2,
			BT_TAB3,
			BT_TAB4,
			BT_SPUP0,
			BT_SPUP1,
			BT_SPUP2,
			BT_SPUP3,
			BT_MACRO
		};

		static constexpr uint16_t TABS = 5;
		static constexpr int16_t ROWS = 4;
		static constexpr int16_t ROW_HEIGHT = 40;

		const CharStats &stats;
		const SkillBook &skillbook;

		ClassicTabs tabs;
		Slider slider;
		Texture skille;
		Texture skilld;
		Texture line;
		Texture recommend;
		Texture bookicon;
		Text booktext;
		Text splabel;

		Job job;
		int16_t sp;
		int16_t beginner_sp;

		uint16_t tab;
		uint16_t offset;

		std::vector<SkillDisplayMeta> skills;
		// The skill v83 recommends raising, or 0.
		int32_t recommended = 0;
		bool grabbing;
	};
}