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
#include "../Graphics/Text.h"

#include "../Character/Look/CharLook.h"

#include "../Net/Handlers/ShopList.h"

namespace ms
{
	class UIShop : public UIDragElement<PosSHOP>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SHOP;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = true;

		UIShop(const CharLook &charlook, const Inventory &inventory);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> position) override;

		// Tabs and rows.
		bool own_cursor(uint16_t buttonid) const override
		{ return buttonid >= Buttons::EQUIP; }

		void send_scroll(double yoffset) override;

		void rightclick(Point<int16_t> cursorpos) override;
		void doubleclick(Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void reset(int32_t npcid);

		void add_item(const shop_list::Row &row);

		void modify(InventoryType::Id type);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void clear_tooltip();

		void show_item(int16_t slot, bool sale);

		void changeselltab(InventoryType::Id tab);

		// The visible row under y, or -1.
		int16_t slot_by_position(int16_t y) const;

		static uint16_t tabbyinventory(InventoryType::Id type);

		void update_recharge();

		// The shop's row for a star or bullet, which it recharges, if any.
		const shop_list::Row *recharge_row(int32_t itemid) const;

		void exit_shop();

		static constexpr int16_t ROWS = 5;

		enum Buttons : int16_t
		{
			BUY_ITEM,
			SELL_ITEM,
			EXIT,
			// Before the rows they sit on, so a click recharges and stops there.
			RECHARGE0,
			RECHARGE4 = RECHARGE0 + ROWS - 1,
			EQUIP,
			USE,
			SETUP,
			ETC,
			CASH,
			BUY0,
			BUY4 = BUY0 + ROWS - 1,
			SELL0,
			SELL4 = SELL0 + ROWS - 1,
			NUM_BUTTONS
		};

		const CharLook &charlook;
		const Inventory &inventory;

		Texture npc;
		Texture selection;
		Texture meso;
		Text mesolabel;

		ClassicTabs buytabs;
		ClassicTabs selltabs;

		Slider buyslider;
		Slider sellslider;

		// Stars and bullets the shop recharges, with their full set.
		std::vector<shop_list::Row> recharges;

		bool rightclicksell;

		Point<int16_t> lastcursorpos;

		class BuyItem
		{
		public:
			BuyItem(Texture cur, const shop_list::Row &row);

			void draw(Point<int16_t> position) const;

			int16_t get_index() const;

			int32_t get_id() const;

			int16_t get_buyable() const;

		private:
			Texture icon;
			Texture currency;
			shop_list::Row row;
			Text namelabel;
			Text pricelabel;
		};

		class SellItem
		{
		public:
			SellItem(int32_t item_id, int16_t count, int16_t slot, bool showcount, bool sealed, Texture cur);

			void draw(Point<int16_t> position) const;

			int32_t get_id() const;

			int16_t get_slot() const;

			int16_t get_sellable() const;

			bool is_sealed() const;

		private:
			Texture icon;
			Texture currency;
			int32_t id;
			int16_t slot;
			int16_t sellable;
			bool showcount;
			bool sealed;
			Text namelabel;
			Text pricelabel;
		};

		struct BuyState
		{
			std::vector<BuyItem> items;
			int16_t offset;
			int16_t lastslot;
			int16_t selection;

			void reset();

			void draw(Point<int16_t> position, const Texture &selected) const;

			void show_item(int16_t slot);

			void add(BuyItem item);

			void buy() const;

			void select(int16_t selected);
		};

		BuyState buystate;

		struct SellState
		{
			std::vector<SellItem> items;
			int16_t offset;
			InventoryType::Id tab;
			int16_t lastslot;
			int16_t selection;

			void reset();

			void change_tab(const Inventory &inventory, InventoryType::Id type, Texture meso);

			void draw(Point<int16_t> position, const Texture &selected) const;

			void show_item(const Inventory &inventory, int16_t slot);

			void sell(bool skip_confirmation) const;

			void select(int16_t selected);
		};

		SellState sellstate;
	};
}