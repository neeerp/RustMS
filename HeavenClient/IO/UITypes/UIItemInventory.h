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

namespace ms
{
	// The Item inventory
	class UIItemInventory : public UIDragElement<PosINV>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMINVENTORY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIItemInventory(const Inventory &inventory);

		void draw(float inter) const override;

		void update() override;

		void doubleclick(Point<int16_t> position) override;

		bool send_icon(const Icon &icon, Point<int16_t> position) override;

		void toggle_active() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool pressed, Point<int16_t> position) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void modify(InventoryType::Id type, int16_t pos, int8_t mode, int16_t arg);

		void set_sort(bool enabled);

		void change_tab(InventoryType::Id type);

		void clear_new();

		// Show a tab's new slot count.
		void refresh_slots();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void show_item(int16_t slot);

		void clear_tooltip();

		void load_icons();

		void update_slot(int16_t slot);

		void update_slider();

		bool is_visible(int16_t slot) const;

		bool can_wear_equip(int16_t slot) const;

		int16_t first_slot() const;

		int16_t last_slot() const;

		int16_t slot_by_position(Point<int16_t> position) const;

		Point<int16_t> get_slotpos(int16_t slot) const;

		Icon *get_icon(int16_t slot);

		void set_full(bool enabled);

		class ItemIcon : public Icon::Type
		{
		public:
			ItemIcon(const UIItemInventory &parent, InventoryType::Id sourcetab, EquipSlot::Id eqsource, int16_t source,
					 int32_t item_id, int16_t count, bool questitem, bool untradable, bool cashitem);

			void drop_on_stage() const override;

			void drop_on_equips(EquipSlot::Id eqslot) const override;

			bool drop_on_items(InventoryType::Id tab, EquipSlot::Id eqslot, int16_t slot, bool equip) const override;

			void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const override;
			Keyboard::Mapping get_mapping() const override;

			void set_count(int16_t count) override;

			Icon::IconType get_type() override;

		private:
			InventoryType::Id sourcetab;
			EquipSlot::Id eqsource;
			int16_t source;
			int32_t item_id;
			int16_t count;
			bool questitem;
			bool untradable;
			// A cash item or one with a Cash Shop serial
			bool cashitem;
			const UIItemInventory &parent;
		};

		static constexpr uint16_t TABS = 5;
		static constexpr int16_t ROWS = 6;
		static constexpr int16_t COLUMNS = 4;
		// The full window shows four pages of ROWS * COLUMNS side by side.
		static constexpr int16_t PAGES = 4;
		static constexpr int16_t MAXSLOTS = ROWS * COLUMNS;
		static constexpr int16_t MAXFULLSLOTS = PAGES * MAXSLOTS;

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_TAB0,
			BT_TAB1,
			BT_TAB2,
			BT_TAB3,
			BT_TAB4,
			BT_COIN,
			BT_GATHER,
			BT_SORT,
			BT_FULL,
			BT_SMALL
		};

		const Inventory &inventory;

		Texture backgrnd;
		Texture full_backgrnd;
		ClassicTabs tabs;
		Animation newitemslot;
		Animation newitemtab[2];
		Texture projectile;
		Texture disabled;
		Text mesolabel;
		Slider slider;

		std::map<int16_t, std::unique_ptr<Icon>> icons;
		// The first row shown in each tab.
		std::map<InventoryType::Id, int16_t> rows;

		InventoryType::Id tab;
		InventoryType::Id newtab;
		int16_t newslot;
		bool ignore_tooltip;

		bool sort_enabled;
		bool full_enabled;
	};
}
