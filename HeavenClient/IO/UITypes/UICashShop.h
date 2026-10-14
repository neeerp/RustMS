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

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Data/CashCatalog.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/Text.h"

#include <array>
#include <string>
#include <vector>

namespace ms
{
	// The v83 Cash Shop (UIClassic.nx CashShop.img), an 800x600 screen: the
	// avatar preview, Cash and Item Inventories on the left, the catalog of
	// Etc.nx Commodity.img in tabs and pages, and the Best Item column.
	class UICashShop : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CASHSHOP;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// An item in the account's Cash Inventory.
		struct CashItem
		{
			int64_t cash_id;
			int32_t item_id;
			int16_t count;
		};

		UICashShop();

		void draw(float inter) const override;
		void update() override;

		Button::State button_pressed(uint16_t buttonid) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		// Double-clicking moves an item between the Cash and Item Inventories.
		void doubleclick(Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		void exit_cashshop();

		// The account's name and the Best Item commodities, from SET_CASH_SHOP.
		void set_account(const std::string& account, const std::vector<int32_t>& best);
		void set_balance(int32_t credit, int32_t points, int32_t prepaid);
		void set_cash_items(const std::vector<CashItem>& items);
		void add_cash_items(const std::vector<CashItem>& items);
		void remove_cash_item(int64_t cash_id);
		void set_wish_list(const std::vector<int32_t>& sns);

	private:
		void change_tab(cash_catalog::Tab tab);
		void change_subtab(size_t subtab);
		void change_page(size_t page);
		void update_list();
		void update_page();
		void update_cash_slider();
		void update_item_slider();

		void draw_item_inventory() const;

		// The cell of a grid under the cursor, or -1.
		int16_t grid_cell(Point<int16_t> cursor, Point<int16_t> grid, size_t columns, size_t rows) const;

		static constexpr size_t LIST_COLUMNS = 2;
		static constexpr size_t BEST_ITEMS = 5;
		static constexpr size_t CASH_COLUMNS = 6;
		static constexpr size_t CASH_ROWS = 2;
		static constexpr size_t ITEM_COLUMNS = 4;
		static constexpr size_t ITEM_ROWS = 3;
		static constexpr size_t ITEM_TABS = 5;

		enum Buttons : uint16_t
		{
			BT_PREVIEW1,
			BT_PREVIEW2,
			BT_PREVIEW3,
			BT_BUY_AVATAR,
			BT_DEFAULT_AVATAR,
			BT_TAKEOFF_AVATAR,
			BT_EX_EQUIP,
			BT_EX_USE,
			BT_EX_SETUP,
			BT_EX_ETC,
			BT_EX_TRUNK,
			BT_SEARCH,
			BT_CHARGE,
			BT_CHECK,
			BT_COUPON,
			BT_EXIT,
			// BUY, GIFT, then RESERVE (or REMOVE on the wish list) per cell
			BT_LIST
		};

		struct Cell
		{
			Text name;
			Text price;
		};

		struct Best
		{
			int32_t item_id;
			Text name;
			Text price;
		};

		std::vector<cash_catalog::Commodity> commodities;
		std::array<std::vector<std::string>, cash_catalog::NUM_TABS> subtab_names;

		Texture background;
		Texture preview_backgrounds[3];
		Texture preview_frame;
		Texture tab_bars[cash_catalog::NUM_TABS];
		Texture item_base;
		Texture how_to[2];
		Texture sold_out;
		Texture item_tab_labels[2][ITEM_TABS];
		std::vector<Animation> labels;

		cash_catalog::Tab tab;
		size_t subtab;
		size_t page;
		uint8_t preview_index;
		bool female;

		std::vector<cash_catalog::Commodity> listed;
		std::vector<Text> subtab_texts;
		std::vector<Text> page_texts;
		std::array<Cell, cash_catalog::PAGE_SIZE> cells;
		std::vector<Best> best_items;
		std::vector<int32_t> wish_list;

		Text name_label;
		Text account_label;
		Text balance_labels[3];
		int32_t credit;
		int32_t points;
		int32_t prepaid;

		std::vector<CashItem> cash_items;
		Slider cash_slider;
		size_t cash_row;

		InventoryType::Id item_tab;
		Slider item_slider;
		size_t item_row;
		ColorBox item_tab_outline;
		ColorBox item_tab_selected;
		ColorBox item_tab_unselected;
	};
}
