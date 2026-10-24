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

#include "../Keyboard.h"
#include "../KeyConfig.h"
#include "../KeyType.h"
#include "../UIDragElement.h"

#include "../../Character/SkillBook.h"
#include "../../Character/SkillMacros.h"

namespace ms
{
	class UIKeyConfig : public UIDragElement<PosKEYCONFIG>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::KEYCONFIG;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIKeyConfig(const Inventory& inventory, const SkillBook& skillbook);

		void draw(float inter) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		bool send_icon(const Icon& icon, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		void toggle_active() override;

		UIElement::Type get_type() const override;

		void close();
		// A close that asks first when keys or quick slots are unsaved
		// (FUN_00833749).
		void safe_close();

		void stage_mapping(Point<int16_t> cursorposition, Keyboard::Mapping mapping);
		void stage_key(int32_t key, Keyboard::Mapping mapping);
		void unstage_mapping(Keyboard::Mapping mapping);

		void update_item_count(InventoryType::Id type, int16_t slot, int16_t change);

		// The icon of what a key is bound to, as the quick slot editor draws
		// it (FUN_0072d162).
		Texture key_icon(int32_t key) const;
		// What a key is bound to with the window's unsaved edits, which v83 makes
		// on the live key map the status bar draws.
		Keyboard::Mapping get_staged_mapping(int32_t keycode) const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The confirmations the window asks over its keyboard, in the
		// order of their bitmaps.
		enum Notice : uint8_t
		{
			RESET,
			CLEAR,
			SAVE,
			NONE
		};

		void load_item_icons();
		void load_skill_icons();

		void show_notice(Notice notice);
		void hide_notice();
		void answer_notice(bool ok);

		void show_item(int32_t item_id);
		void show_skill(int32_t skill_id);
		void clear_tooltip();

		void save_staged_mappings();
		void send_quick_slots();
		void refresh_bound_actions();
		void clear();
		void reset();

		Icon* mapping_icon(Keyboard::Mapping mapping) const;
		std::unique_ptr<Icon> item_icon(Keyboard::Mapping mapping) const;
		Texture get_item_texture(int32_t item_id) const;
		Texture get_skill_texture(int32_t skill_id) const;
		int32_t key_by_position(Point<int16_t> position) const;
		KeyAction::Id unbound_action_by_position(Point<int16_t> position) const;
		bool is_action_mapping(Keyboard::Mapping mapping) const;
		bool is_bound(KeyAction::Id action) const;
		static KeyType::Id get_keytype(KeyAction::Id action);

		enum Buttons : uint16_t
		{
			CLOSE,
			DEFAULT,
			DELETE,
			QUICKSLOT,
			OK,
			CANCEL,
			NOTICE_OK,
			NOTICE_CANCEL
		};

		class MappingIcon : public Icon::Type
		{
		public:
			MappingIcon(Keyboard::Mapping mapping);
			MappingIcon(KeyAction::Id keyId);

			void drop_on_stage() const override;
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const override;
			void set_count(int16_t) override {}
			Icon::IconType get_type() override;

		private:
			Keyboard::Mapping mapping;
		};

		// Used for displaying item counts
		class CountableMappingIcon : public MappingIcon
		{
		public:
			CountableMappingIcon(Keyboard::Mapping mapping, int16_t count);

			void set_count(int16_t count) override;

		private:
			int16_t count;
		};

		const Inventory& inventory;
		const SkillBook& skillbook;

		bool dirty;
		Notice notice;

		Keyboard* keyboard = nullptr;

		Texture notices[Notice::NONE];
		std::map<int32_t, Texture> key_labels;

		std::map<KeyAction::Id, std::unique_ptr<Icon>> action_icons;
		std::map<KeyAction::Id, Point<int16_t>> unbound_actions_pos;

		std::map<int32_t, std::unique_ptr<Icon>> item_icons;
		std::map<int32_t, std::unique_ptr<Icon>> skill_icons;
		// SkillMacro/Macroicon, for each place a macro can have.
		std::unique_ptr<Icon> macro_icons[SkillMacros::MAX_MACROS];

		// Used to determine if mapping belongs to predefined action, e.g. attack, pick up, faces, etc.
		std::vector<Keyboard::Mapping> action_mappings;

		std::vector<KeyAction::Id> bound_actions;
		std::map<int32_t, Keyboard::Mapping> staged_mappings;
	};
}
