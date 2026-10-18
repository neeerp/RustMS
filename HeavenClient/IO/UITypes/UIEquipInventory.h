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

#include "../../Graphics/Geometry.h"

namespace ms
{
	// The Equip inventory, with the pet equipment panel that opens beside it
	class UIEquipInventory : public UIDragElement<PosEQINV>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::EQUIPINVENTORY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIEquipInventory(const Inventory &inventory);

		void draw(float inter) const override;

		void toggle_active() override;
		bool send_icon(const Icon& icon, Point<int16_t> position) override;

		void doubleclick(Point<int16_t> position) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool pressed, Point<int16_t> position) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void modify(int16_t pos, int8_t mode, int16_t arg);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Shows the tooltip for worn position `slot`.
		void show_equip(int16_t slot);
		void clear_tooltip();

		void load_icons();
		void update_slot(EquipSlot::Id slot);
		bool is_unavailable(EquipSlot::Id slot) const;
		EquipSlot::Id slot_by_position(Point<int16_t> position) const;
		void show_pet(bool show);

		class EquipIcon : public Icon::Type
		{
		public:
			// For the item at worn position `source`.
			EquipIcon(int16_t source);

			void drop_on_stage() const override;
			void drop_on_equips(EquipSlot::Id slot) const override;
			bool drop_on_items(InventoryType::Id tab, EquipSlot::Id eqslot, int16_t slot, bool equip) const override;
			void drop_on_bindings(Point<int16_t>, bool) const override {}
			void set_count(int16_t) override {}
			Icon::IconType get_type() override;

		private:
			int16_t source;
		};

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_PET_SHOW,
			BT_PET_HIDE,
			BT_PET1,
			BT_PET2,
			BT_PET3
		};

		const Inventory &inventory;

		EnumMap<EquipSlot::Id, Point<int16_t>> iconpositions;
		EnumMap<EquipSlot::Id, std::unique_ptr<Icon>> icons;

		Texture backgrnd;
		Texture pet;
		ColorBox unavailable;

		bool pet_enabled;
	};
}
