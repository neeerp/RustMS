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

#include "../../Graphics/Texture.h"

#include "../../Character/Inventory/Inventory.h"
#include "../Keyboard.h"

#include <memory>

namespace ms
{
	class Icon
	{
	public:
		enum IconType : uint8_t
		{
			NONE,
			SKILL,
			EQUIP,
			ITEM,
			KEY,
			// A skill taken out of a macro's slot.
			MACRO_SKILL,
			// A macro, to bind to a key.
			MACRO,
			NUM_TYPES
		};

		class Type
		{
		public:
			virtual ~Type()
			{}

			virtual void drop_on_stage() const = 0;

			virtual void drop_on_equips(EquipSlot::Id eqslot) const = 0;

			virtual bool drop_on_items(InventoryType::Id tab, EquipSlot::Id eqslot, int16_t slot, bool equip) const = 0;

			virtual void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const = 0;

			// Offers the icon's item in one of my trade slots (1 to 9).
			virtual void drop_on_trade(int8_t slot) const {}

			// Puts the icon's item up for sale in my personal shop.
			virtual void drop_on_shop() const {}

			// Shows the icon's item in the item megaphone's window.
			virtual void drop_on_megaphone() const {}

			// The inventory tab an icon from another window goes back to,
			// or NONE if the inventory doesn't take it.
			virtual InventoryType::Id get_return_tab() const
			{
				return InventoryType::Id::NONE;
			}

			// Hands the icon back to the inventory, as a shop row taken back.
			virtual void drop_on_inventory() const {}

			// What a key bound to this icon would do; NONE if it cannot be
			// bound.
			virtual Keyboard::Mapping get_mapping() const
			{
				return {};
			}

			virtual void set_count(int16_t) = 0;

			virtual IconType get_type() = 0;
		};

		class NullType : public Type
		{
			void drop_on_stage() const override {}
			void drop_on_equips(EquipSlot::Id) const override {}
			bool drop_on_items(InventoryType::Id, EquipSlot::Id, int16_t, bool) const override { return true; }
			void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const override {}
			void set_count(int16_t) override {}
			IconType get_type() override { return IconType::NONE; }
		};

		Icon(std::unique_ptr<Type> type, Texture texture, int16_t count);

		Icon();

		void drop_on_stage() const;
		void drop_on_equips(EquipSlot::Id eqslot) const;
		bool drop_on_items(InventoryType::Id tab, EquipSlot::Id eqslot, int16_t slot, bool equip) const;
		void drop_on_bindings(Point<int16_t> cursorposition, bool remove) const;
		void drop_on_trade(int8_t slot) const;
		void drop_on_shop() const;
		void drop_on_megaphone() const;
		InventoryType::Id get_return_tab() const;
		void drop_on_inventory() const;
		Keyboard::Mapping get_mapping() const;

		void set_count(int16_t count);

		IconType get_type() const;

		void draw(Point<int16_t> position) const;

		void dragdraw(Point<int16_t> cursorpos) const;

		void start_drag(Point<int16_t> offset);

		void reset();

		virtual Texture get_texture() const;

		int16_t get_count() const;

		bool get_drag();

	private:
		std::unique_ptr<Type> type;
		bool showcount;
		int16_t count;

		Texture texture;
		bool dragged;
		Point<int16_t> cursoroffset;
	};
}