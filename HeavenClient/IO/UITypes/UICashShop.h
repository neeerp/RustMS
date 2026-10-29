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

#include "../Components/EquipTooltip.h"
#include "../Components/ItemTooltip.h"
#include "../Components/PackageTooltip.h"
#include "../Components/Slider.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Character/Look/CharLook.h"
#include "../../Data/CashCatalog.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/Text.h"

#include <array>
#include <map>
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
			// Who sent it, if it was a present.
			std::string gift_from;
			int64_t expiration = 0;
			// The commodity it was bought as.
			int32_t sn = 0;
		};

		UICashShop();

		void draw(float inter) const override;
		void update() override;

		Button::State button_pressed(uint16_t buttonid) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		// Double-clicking moves an item between the Cash and Item Inventories.
		void doubleclick(Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		UIElement::Type get_type() const override;

		void exit_cashshop();
		// Esc asks first (v83 FUN_004bcc86, StringPool 0x250); the Exit
		// button leaves at once.
		void confirm_exit();

		// The commodities SET_CASH_SHOP changes or adds (report b179).
		void apply_changes(const std::vector<cash_catalog::Change>& changes);
		// The account's name and the Best Item commodities, from SET_CASH_SHOP.
		void set_account(const std::string& account, const std::vector<int32_t>& best);
		void set_balance(int32_t credit, int32_t points, int32_t prepaid);
		void set_cash_items(const std::vector<CashItem>& items);
		void add_cash_items(const std::vector<CashItem>& items);
		// A bag gained slots.
		void refresh_slots();
		void set_storage_slots(int16_t slots);
		int16_t get_storage_slots() const;
		// The account's character slots (v83 CCashShop+0x490).
		void set_character_slots(int16_t slots);
		int16_t get_character_slots() const;
		// Take an item off the Cash Inventory, returning its item id, or 0 if it wasn't there.
		int32_t remove_cash_item(int64_t cash_id);
		void set_wish_list(const std::vector<int32_t>& sns);
		// Whether the last change to the wish list added a wish (v83 CCashShop+0x4c8 clear), which
		// v83 announces when the list comes back.
		bool added_wish() const;

		// v83 (CCashShop+0x18) has one request out at a time and ignores the
		// buttons until its answer. Returns whether one may be sent, counting
		// it as sent.
		bool request();
		// A balance or an operation result came.
		void answered();
		// Something was bought with mesos: show its tab and row in the Item
		// Inventory (v83 47b603). v83 also selects the slot.
		void bought_with_mesos(int32_t item_id, int16_t slot);

		// The Name Change commodity whose rules were asked for (v83
		// CCashShop+0x4e4).
		void set_name_change(int32_t sn);
		int32_t get_name_change() const;

		// Whether the character has the item, counting the Cash Inventory
		// with `locker` (v83 4729b5), or any of a package's items.
		bool owns(int32_t item_id, bool locker) const;
		bool owns_package(int32_t item_id) const;

		// Back to the map the player came from.
		void leave();

		// Buy Avatar buys the mannequin's commodities one at a time, the last
		// first, then sums up (v83 46ba55, CCashShop+0x4cc to +0x4d8).
		bool buying_avatar() const;
		// One of them is bought; the next waits for the balance.
		void avatar_item_bought();
		// Buy the next one, or say how it went.
		void buy_next_avatar_item();
		// Start buying these, paid with the balance.
		void buy_avatar(const std::vector<int32_t>& sns, int32_t payment);
		// A failure that ends the purchases early.
		void stop_buying_avatar();

	private:
		void change_tab(cash_catalog::Tab tab);
		// Add a wish, or say why not.
		void reserve(int32_t sn);
		// Buy a commodity that costs mesos, after asking (v83 46e5c5).
		void buy_with_mesos(const cash_catalog::Commodity& commodity);
		// Gift a commodity, after asking (v83 46e822).
		void gift(const cash_catalog::Commodity& commodity);
		// Move an item between the Cash and Item Inventories, unless v83
		// (472632 and 472820) keeps it where it is.
		void take_out(const CashItem& item);
		void put_back(int16_t slot);
		void change_subtab(size_t subtab);
		void change_page(size_t page);
		void update_list();
		void update_page();
		void update_cash_slider();
		void update_item_slider();

		void draw_item_inventory() const;

		// The fitting-room mannequin (v83 CCashShop+0x554): a copy of the
		// player's look that tries commodities on (4bb37c, 4ac3de), and
		// Buy Avatar's checks before it buys them (4696b1).
		void try_on(const cash_catalog::Commodity& commodity);
		// Whether the item went on, or came off when it was on already.
		bool try_on_item(int32_t item_id, int32_t sn, bool packaged);
		void take_off(int16_t slot);
		void default_avatar();
		void take_off_avatar();
		void buy_avatar();

		// The tooltip of the commodity, Cash Inventory item or cash item under the cursor (report 202).
		void update_tooltip(Point<int16_t> cursor, Point<int16_t> cursorpos);
		void clear_tooltip();

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
			cash_catalog::Commodity commodity;
			Text name;
			Text price;
		};

		std::vector<cash_catalog::Commodity> commodities;
		// Every commodity, on sale or not, for what a package's contents cost on their own.
		std::vector<cash_catalog::Commodity> catalog;
		// Where the commodities that cost mesos are listed this time.
		std::vector<cash_catalog::Placement> mesos_placed;
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
		bool pending;
		bool wish_added;
		int32_t name_change_sn;
		int16_t storage_slots;
		int16_t character_slots;

		std::vector<CashItem> cash_items;
		Slider cash_slider;
		size_t cash_row;

		InventoryType::Id item_tab;
		Slider item_slider;
		size_t item_row;
		ColorBox item_tab_outline;
		ColorBox item_tab_selected;
		ColorBox item_tab_unselected;

		EquipTooltip eqtooltip;
		ItemTooltip ittooltip;
		PackageTooltip packagetooltip;
		const Tooltip* tooltip = nullptr;
		Point<int16_t> tooltip_pos;
		// The commodity on the page whose tooltip is shown, kept while the cursor stays on it.
		int16_t hovered = -1;
		// The Cash Inventory item whose tooltip is shown.
		int64_t hovered_cash = 0;

		// The mannequin's look and what it tried on: the item and commodity
		// in each slot, a cash weapon in the weapon's. A package on it (v83
		// CCashShop+0x4dc) can't be bought this way.
		struct Tried
		{
			int32_t item_id;
			int32_t sn;
		};
		CharLook preview_look;
		std::map<int16_t, Tried> tried;
		bool package_on = false;

		// Buy Avatar's commodities, those bought (as indices), how many are
		// left, and the balance paying (0 when nothing is being bought).
		std::vector<int32_t> avatar_sns;
		std::vector<size_t> avatar_bought;
		size_t avatar_left = 0;
		int32_t avatar_payment = 0;
	};
}
