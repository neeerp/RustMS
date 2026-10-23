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
#include "UICashShop.h"

#include "UICashBuy.h"
#include "UICashGift.h"
#include "UICouponDialog.h"
#include "UINotice.h"

#include "../UI.h"
#include "../Window.h"

#include "../Components/MapleButton.h"
#include "../Components/TwoSpriteButton.h"

#include "../../Character/Marriage.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/LoginPackets.h"

#include "Timer.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		using namespace cash_catalog;

		// Window coordinates, measured on the 800x600 art and period footage.
		const Point<int16_t> PREVIEW_POS = Point<int16_t>(24, 40);
		const Point<int16_t> PREVIEW_FRAME_POS = Point<int16_t>(15, 8);
		const Point<int16_t> AVATAR_POS = Point<int16_t>(50, 176);
		const Point<int16_t> TAB_BAR_POS = Point<int16_t>(274, 17);
		const Point<int16_t> SUBTAB_POS = Point<int16_t>(283, 75);
		const Point<int16_t> LIST_POS = Point<int16_t>(277, 97);
		const Point<int16_t> CELL_STEP = Point<int16_t>(206, 81);
		const Point<int16_t> PAGES_POS = Point<int16_t>(480, 505);
		const Point<int16_t> BEST_POS = Point<int16_t>(697, 158);
		const int16_t BEST_STEP = 69;
		// v83's Best Item window; each item is 90 x 68 of its 69 px step.
		const Point<int16_t> BEST_WINDOW = Point<int16_t>(690, 157);
		const int16_t BEST_WIDTH = 90;
		const Point<int16_t> CASH_GRID_POS = Point<int16_t>(21, 349);
		const Point<int16_t> ITEM_GRID_POS = Point<int16_t>(23, 481);
		const int16_t GRID_STEP = 35;
		// v83 hit-tests the icons for tooltips: 32x32 in the grids, the 2x 64x64 in the list.
		const int16_t ICON_SIZE = 32;
		const Point<int16_t> LIST_ICON = Point<int16_t>(5, 5);
		const int16_t LIST_ICON_SIZE = 64;
		// The list window, which keeps a commodity's tooltip until the cursor leaves it.
		const Point<int16_t> LIST_WINDOW = Point<int16_t>(275, 95);
		const Point<int16_t> LIST_SIZE = Point<int16_t>(412, 430);
		// Tooltips open 20 px right of the cursor in the list and the Cash Inventory, and under it in the Item
		// Inventory, as in game (UIStateGame's cursor + (0, 22)).
		const Point<int16_t> TOOLTIP_RIGHT = Point<int16_t>(20, 22);
		// A package's tooltip has its top left here from the cursor (FUN_008efcb7).
		const Point<int16_t> PACKAGE_TOOLTIP = Point<int16_t>(20, 20);
		const Point<int16_t> TOOLTIP_BELOW = Point<int16_t>(0, 22);
		const Point<int16_t> ITEM_TABS_POS = Point<int16_t>(21, 457);
		const Point<int16_t> ITEM_TAB_SIZE = Point<int16_t>(30, 16);
		const int16_t ITEM_TAB_STEP = 31;

		// Tabs are 53 pixels apart on CSTab/Tab, except the wider WISH LIST.
		constexpr int16_t TAB_WIDTH = 53;
		constexpr int16_t WISH_LIST_LEFT = 440;

		// An icon hangs from the bottom-left of its 32x32 square.
		const Point<int16_t> ICON_OFFSET = Point<int16_t>(0, 32);

		Text::Font subtab_font(bool selected)
		{
			return selected ? Text::Font::A12B : Text::Font::A12M;
		}

		// The light green EVENT bar has black sub-tabs, the others white, as
		// in period footage.
		Color::Name subtab_color(Tab tab)
		{
			return tab == EVENT_TAB ? Color::Name::BLACK : Color::Name::WHITE;
		}

		std::string price_text(const Commodity& commodity)
		{
			std::string price = std::to_string(commodity.price);
			string_format::split_number(price);
			price += " NX Cash";

			if (commodity.count > 1)
				price += "(" + std::to_string(commodity.count) + ")";

			return price;
		}
	}

	UICashShop::UICashShop() : tab(EVENT_TAB), subtab(0), page(0), preview_index(0), female(false), credit(0), points(0), prepaid(0), pending(false), wish_added(false), name_change_sn(0), storage_slots(0), cash_row(0), item_tab(InventoryType::Id::EQUIP), item_row(0)
	{
		nl::node CashShop = NxFiles::ui_classic["CashShop.img"];
		nl::node Base = CashShop["Base"];
		nl::node CSTab = CashShop["CSTab"];
		nl::node CSList = CashShop["CSList"];
		nl::node CSChar = CashShop["CSChar"];
		nl::node CSInventory = CashShop["CSInventory"];
		nl::node CSStatus = CashShop["CSStatus"];
		nl::node CSEffect = CashShop["CSEffect"];
		nl::node PicturePlate = CashShop["PicturePlate"];

		const CharStats& stats = Stage::get().get_player().get_stats();
		uint16_t job = stats.get_job().get_id();
		female = stats.get_female();

		// Cygnus Knights and Aran each have their own background.
		if (job / 1000 == 1)
			background = Base["backgrnd1"];
		else if (job == 2000 || job / 100 == 21)
			background = Base["backgrnd2"];
		else
			background = Base["backgrnd"];

		for (size_t i = 0; i < 3; i++)
			preview_backgrounds[i] = Base["Preview"][i];

		preview_frame = Base["PreviewOnOff"]["On"]["0"];

		for (size_t i = 0; i < NUM_TABS; i++)
			tab_bars[i] = CSTab["Tab"][std::to_string(i + 1)];

		item_base = CSList["Base"];
		how_to[0] = PicturePlate["HowTo"];
		how_to[1] = PicturePlate["HowToGift"];
		sold_out = PicturePlate["NoItem"];

		for (size_t i = 0; i < ITEM_TABS; i++)
		{
			item_tab_labels[false][i] = Base["Tab2"]["Disable"][i];
			item_tab_labels[true][i] = Base["Tab2"]["Enable"][i];
		}

		// In the order of cash_catalog::Label.
		for (const char* label : { "new", "sale", "hot", "event" })
			labels.emplace_back(CSEffect[label]);

		const int16_t preview_x[3] = { 149, 184, 216 };

		// A tab strip (CCtrlTab, FUN_004ab0b0): selects as the mouse goes down.
		for (size_t i = 0; i < 3; i++)
		{
			buttons[Buttons::BT_PREVIEW1 + i] = std::make_unique<TwoSpriteButton>(Base["Tab"]["Disable"][i], Base["Tab"]["Enable"][i], Point<int16_t>(preview_x[i], 18), Point<int16_t>(preview_x[i], 11));
			buttons[Buttons::BT_PREVIEW1 + i]->set_tab();
		}

		buttons[Buttons::BT_PREVIEW1]->set_state(Button::State::PRESSED);

		buttons[Buttons::BT_BUY_AVATAR] = std::make_unique<MapleButton>(CSChar["BtBuyAvatar"], Point<int16_t>(17, 238));
		buttons[Buttons::BT_DEFAULT_AVATAR] = std::make_unique<MapleButton>(CSChar["BtDefaultAvatar"], Point<int16_t>(102, 238));
		buttons[Buttons::BT_TAKEOFF_AVATAR] = std::make_unique<MapleButton>(CSChar["BtTakeoffAvatar"], Point<int16_t>(188, 238));

		const char* expansions[5] = { "BtExEquip", "BtExConsume", "BtExInstall", "BtExEtc", "BtExTrunk" };

		for (size_t i = 0; i < 5; i++)
			buttons[Buttons::BT_EX_EQUIP + i] = std::make_unique<MapleButton>(CSInventory[expansions[i]], Point<int16_t>(178, 452 + 27 * i));

		buttons[Buttons::BT_SEARCH] = std::make_unique<MapleButton>(CashShop["CSItemSearch"]["BtSearch"], Point<int16_t>(691, 101));
		buttons[Buttons::BT_CHARGE] = std::make_unique<MapleButton>(CSStatus["BtCharge"], Point<int16_t>(505, 544));
		buttons[Buttons::BT_CHECK] = std::make_unique<MapleButton>(CSStatus["BtCheck"], Point<int16_t>(545, 544));
		buttons[Buttons::BT_COUPON] = std::make_unique<MapleButton>(CSStatus["BtCoupon"], Point<int16_t>(586, 544));
		buttons[Buttons::BT_EXIT] = std::make_unique<MapleButton>(CSStatus["BtExit"], Point<int16_t>(632, 546));

		// The avatar's buttons' help hangs from their top right, the status
		// bar's sits on it (0x4ab0b0, 0x4bc3e4). The inventory expansions'
		// help is never read.
		set_button_help(Buttons::BT_BUY_AVATAR, "Shop/Button/BuyAvatar");
		set_button_help(Buttons::BT_DEFAULT_AVATAR, "Shop/Button/DefaultAvatar");
		set_button_help(Buttons::BT_TAKEOFF_AVATAR, "Shop/Button/TakeoffAvatar");
		set_button_help(Buttons::BT_CHARGE, "Shop/Button/Charge", true);
		set_button_help(Buttons::BT_CHECK, "Shop/Button/Check", true);
		set_button_help(Buttons::BT_COUPON, "Shop/Button/Coupon", true);

		for (size_t i = 0; i < PAGE_SIZE; i++)
		{
			div_t div = std::div(static_cast<int>(i), static_cast<int>(LIST_COLUMNS));
			Point<int16_t> cell = LIST_POS + Point<int16_t>(CELL_STEP.x() * div.rem, CELL_STEP.y() * div.quot);
			uint16_t first = Buttons::BT_LIST + 3 * i;

			buttons[first] = std::make_unique<MapleButton>(CSList["BtBuy"], cell + Point<int16_t>(77, 54));
			buttons[first + 1] = std::make_unique<MapleButton>(CSList["BtGift"], cell + Point<int16_t>(117, 54));
			buttons[first + 2] = std::make_unique<MapleButton>(CSList["BtReserve"], cell + Point<int16_t>(157, 54));
			buttons[Buttons::BT_LIST + 3 * PAGE_SIZE + i] = std::make_unique<MapleButton>(CSList["BtRemove"], cell + Point<int16_t>(157, 54));

			cells[i].name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
			cells[i].price = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		}

		for (nl::node sub : nl::nx::etc["Category.img"])
		{
			int64_t category = sub["Category"].get_integer();
			int64_t index = sub["CategorySub"].get_integer();
			Tab owner = category == MAIN_CATEGORY ? MAIN : static_cast<Tab>(category);

			if (category < 1 || category > MAIN_CATEGORY || index < 0)
				continue;

			std::vector<std::string>& names = subtab_names[owner];

			if (names.size() <= static_cast<size_t>(index))
				names.resize(index + 1);

			names[index] = sub["Name"].get_string();
		}

		for (nl::node item : nl::nx::etc["Commodity.img"])
		{
			int32_t sn = static_cast<int32_t>(item["SN"].get_integer());

			catalog.push_back({
				sn,
				static_cast<int32_t>(item["ItemId"].get_integer()),
				static_cast<int32_t>(item["Price"].get_integer()),
				static_cast<int16_t>(item["Count"].get_integer(1)),
				static_cast<int8_t>(item["Gender"].get_integer(2)),
				label(static_cast<int32_t>(item["Class"].get_integer(-1)), sn),
				static_cast<int32_t>(item["Priority"].get_integer()),
				static_cast<int32_t>(item["Period"].get_integer())
			});

			if (item["OnSale"].get_integer() == 1)
				commodities.push_back(catalog.back());
		}

		name_label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK, stats.get_name());
		account_label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		for (size_t i = 0; i < 3; i++)
			balance_labels[i] = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::BLACK, "0");

		item_tab_outline = ColorBox(ITEM_TAB_SIZE.x(), ITEM_TAB_SIZE.y(), Color::Name::BOULDER, 1.0f);
		item_tab_selected = ColorBox(ITEM_TAB_SIZE.x() - 2, ITEM_TAB_SIZE.y() - 2, Color::Name::CHARM, 1.0f);
		item_tab_unselected = ColorBox(ITEM_TAB_SIZE.x() - 2, ITEM_TAB_SIZE.y() - 2, Color::Name::SILVERCHALICE, 1.0f);

		cash_slider = Slider(
			Slider::Type::CLASSIC, Range<int16_t>(347, 416), 229, CASH_ROWS, 1,
			[&](bool upwards)
			{
				size_t rows = (cash_items.size() + CASH_COLUMNS - 1) / CASH_COLUMNS;

				if (upwards && cash_row > 0)
					cash_row--;
				else if (!upwards && cash_row + CASH_ROWS < rows)
					cash_row++;
			}
		);

		item_slider = Slider(
			Slider::Type::CLASSIC, Range<int16_t>(476, 584), 163, ITEM_ROWS, 1,
			[&](bool upwards)
			{
				const Inventory& inventory = Stage::get().get_player().get_inventory();
				size_t rows = (inventory.get_slotmax(item_tab) + ITEM_COLUMNS - 1) / ITEM_COLUMNS;

				if (upwards && item_row > 0)
					item_row--;
				else if (!upwards && item_row + ITEM_ROWS < rows)
					item_row++;
			}
		);

		update_cash_slider();
		update_item_slider();
		change_tab(EVENT_TAB);

		dimension = background.get_dimensions();
	}

	void UICashShop::draw(float inter) const
	{
		// The preview shows through the background's transparent hole.
		preview_backgrounds[preview_index].draw(position + PREVIEW_POS);
		background.draw(position);
		preview_frame.draw(position + PREVIEW_FRAME_POS);

		const Player& player = Stage::get().get_player();
		player.get_look().draw(position + AVATAR_POS, false, Stance::Id::STAND1, Expression::Id::DEFAULT);

		Text nametag(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG, player.get_stats().get_name());
		nametag.draw(position + AVATAR_POS + Point<int16_t>(0, 4));

		name_label.draw(position + Point<int16_t>(238, 268));
		account_label.draw(position + Point<int16_t>(238, 286));

		tab_bars[tab].draw(position + TAB_BAR_POS);

		Point<int16_t> subtab_pos = position + SUBTAB_POS;

		for (const Text& text : subtab_texts)
		{
			text.draw(subtab_pos);
			subtab_pos.shift_x(text.width());
		}

		if (tab == MAIN)
		{
			if (subtab < 2)
				how_to[subtab].draw(position + LIST_POS);
		}
		else if (listed.empty() && tab != WISH_LIST)
		{
			sold_out.draw(position + LIST_POS);
		}
		else
		{
			for (size_t i = 0; i < PAGE_SIZE; i++)
			{
				size_t index = page * PAGE_SIZE + i;

				if (index >= listed.size())
					break;

				div_t div = std::div(static_cast<int>(i), static_cast<int>(LIST_COLUMNS));
				Point<int16_t> cell = position + LIST_POS + Point<int16_t>(CELL_STEP.x() * div.rem, CELL_STEP.y() * div.quot);
				const Commodity& commodity = listed[index];

				item_base.draw(cell);
				ItemData::get(commodity.item_id).get_icon(false).draw(DrawArgument(cell + Point<int16_t>(5, 69), 2.0f, 2.0f));

				if (commodity.label != cash_catalog::NONE)
					labels[commodity.label].draw(cell + Point<int16_t>(3, -2), inter);

				cells[i].name.draw(cell + Point<int16_t>(78, 5));
				cells[i].price.draw(cell + Point<int16_t>(78, 30));
			}

			Point<int16_t> page_pos = position + PAGES_POS;
			int16_t width = 0;

			for (const Text& text : page_texts)
				width += text.width();

			page_pos.shift_x(-width / 2);

			for (const Text& text : page_texts)
			{
				text.draw(page_pos);
				page_pos.shift_x(text.width());
			}
		}

		for (size_t i = 0; i < best_items.size(); i++)
		{
			Point<int16_t> pos = position + BEST_POS + Point<int16_t>(0, BEST_STEP * i);

			ItemData::get(best_items[i].commodity.item_id).get_icon(false).draw(pos + Point<int16_t>(45, 1) + ICON_OFFSET);
			best_items[i].name.draw(pos + Point<int16_t>(0, 31));
			best_items[i].price.draw(pos + Point<int16_t>(0, 48));
		}

		for (size_t i = 0; i < CASH_COLUMNS * CASH_ROWS; i++)
		{
			size_t index = cash_row * CASH_COLUMNS + i;

			if (index >= cash_items.size())
				break;

			Point<int16_t> cell = CASH_GRID_POS + Point<int16_t>(GRID_STEP * (i % CASH_COLUMNS), GRID_STEP * (i / CASH_COLUMNS));
			ItemData::get(cash_items[index].item_id).get_icon(false).draw(position + cell + ICON_OFFSET);
		}

		draw_item_inventory();

		Point<int16_t> balance_pos = position + Point<int16_t>(480, 541);

		for (size_t i = 0; i < 3; i++)
			balance_labels[i].draw(balance_pos + Point<int16_t>(0, 14 * i));

		cash_slider.draw(position);
		item_slider.draw(position);

		UIElement::draw_buttons(inter);

		if (tooltip)
			tooltip->draw(tooltip_pos);
	}

	void UICashShop::draw_item_inventory() const
	{
		for (size_t i = 0; i < ITEM_TABS; i++)
		{
			bool selected = item_tab == InventoryType::Id::EQUIP + i;
			Point<int16_t> pos = position + ITEM_TABS_POS + Point<int16_t>(ITEM_TAB_STEP * i, 0);
			const Texture& text = item_tab_labels[selected][i];

			item_tab_outline.draw(pos);
			(selected ? item_tab_selected : item_tab_unselected).draw(pos + Point<int16_t>(1, 1));
			text.draw(pos + (ITEM_TAB_SIZE - text.get_dimensions()) / 2);
		}

		const Inventory& inventory = Stage::get().get_player().get_inventory();

		for (size_t i = 0; i < ITEM_COLUMNS * ITEM_ROWS; i++)
		{
			int16_t slot = static_cast<int16_t>(item_row * ITEM_COLUMNS + i + 1);
			int32_t item_id = inventory.get_item_id(item_tab, slot);

			if (item_id == 0)
				continue;

			Point<int16_t> cell = ITEM_GRID_POS + Point<int16_t>(GRID_STEP * (i % ITEM_COLUMNS), GRID_STEP * (i / ITEM_COLUMNS));
			ItemData::get(item_id).get_icon(false).draw(position + cell + ICON_OFFSET);
		}
	}

	void UICashShop::update()
	{
		UIElement::update();

		for (Animation& label : labels)
			label.update();
	}

	Button::State UICashShop::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_PREVIEW1:
		case Buttons::BT_PREVIEW2:
		case Buttons::BT_PREVIEW3:
			buttons[Buttons::BT_PREVIEW1 + preview_index]->set_state(Button::State::NORMAL);
			preview_index = static_cast<uint8_t>(buttonid - Buttons::BT_PREVIEW1);

			return Button::State::PRESSED;
		case Buttons::BT_EXIT:
			leave();

			return Button::State::NORMAL;
		case Buttons::BT_COUPON:
			// v83 (0x471240) ignores it while a request is out.
			if (!pending)
				UI::get().emplace<UICouponDialog>();

			return Button::State::NORMAL;
		case Buttons::BT_CHECK:
			if (request())
				CheckCashPacket().dispatch();

			return Button::State::NORMAL;
		default:
			break;
		}

		if (buttonid >= Buttons::BT_LIST)
		{
			// Three buttons to a cell, then each cell's REMOVE.
			size_t list_button = buttonid - Buttons::BT_LIST;
			bool remove = list_button >= 3 * PAGE_SIZE;
			size_t index = page * PAGE_SIZE + (remove ? list_button - 3 * PAGE_SIZE : list_button / 3);

			if (index < listed.size())
			{
				const Commodity& commodity = listed[index];

				if (pending)
					return Button::State::NORMAL;

				if (remove && request())
				{
					wish_added = false;
					SetWishListPacket(unwish(wish_list, commodity.sn)).dispatch();
				}
				// BUY or GIFT, a ring of a pair is for two (v83 472fb6).
				else if (list_button % 3 == 0 || (list_button % 3 == 1 && marriage::is_pair_ring(commodity.item_id)))
					UICashBuy::open(commodity, credit, points, prepaid);
				else if (list_button % 3 == 1)
					UI::get().emplace<UICashGift>(commodity);
				else if (list_button % 3 == 2)
					reserve(commodity.sn);
			}
		}

		if (buttonid >= Buttons::BT_EX_EQUIP && buttonid <= Buttons::BT_EX_TRUNK && !pending)
		{
			// Four slots for 4,000 NX, shown as the expansion item: 9111000
			// to 9114000 for the bags, 9110000 for the storage.
			int32_t bag = buttonid == Buttons::BT_EX_TRUNK ? 0 : buttonid - Buttons::BT_EX_EQUIP + 1;
			Commodity slots = {};
			slots.item_id = 9110000 + 1000 * bag;
			slots.price = EXPANSION_PRICE;
			slots.count = 1;
			slots.label = Label::NONE;

			UICashBuy::open(slots, credit, points, prepaid);
		}

		return Button::State::NORMAL;
	}

	Cursor::State UICashShop::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor = cursorpos - position;

		update_tooltip(cursor, cursorpos);

		for (Slider* slider : { &cash_slider, &item_slider })
		{
			if (slider->isenabled())
			{
				Cursor::State state = slider->send_cursor(cursor, clicked);

				if (state != Cursor::State::IDLE)
					return state;
			}
		}

		// The tabs along the top of CSTab/Tab.
		Point<int16_t> in_bar = cursor - TAB_BAR_POS;

		if (in_bar.y() >= 10 && in_bar.y() < 56 && in_bar.x() >= 0 && in_bar.x() < tab_bars[0].width())
		{
			Tab clicked_tab = in_bar.x() >= WISH_LIST_LEFT ? WISH_LIST : static_cast<Tab>(std::min<int16_t>(in_bar.x() / TAB_WIDTH, PACKAGE));

			// v83's bar (FUN_004d9adb) clicks only when the tab changes.
			if (clicked && clicked_tab != tab)
			{
				Sound(Sound::Name::BUTTONCLICK).play();
				change_tab(clicked_tab);
			}

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		// The sub-tabs, laid out left to right in the bar.
		if (cursor.y() >= SUBTAB_POS.y() && cursor.y() < SUBTAB_POS.y() + 16)
		{
			int16_t x = SUBTAB_POS.x();

			for (size_t i = 0; i < subtab_texts.size(); i++)
			{
				int16_t width = subtab_texts[i].width();

				if (cursor.x() >= x && cursor.x() < x + width)
				{
					if (clicked && i != subtab)
					{
						Sound(Sound::Name::BUTTONCLICK).play();
						change_subtab(i);
					}

					return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
				}

				x += width;
			}
		}

		// The page numbers under the list; every other text is a separator.
		if (tab != MAIN && cursor.y() >= PAGES_POS.y() && cursor.y() < PAGES_POS.y() + 16)
		{
			int16_t width = 0;

			for (const Text& text : page_texts)
				width += text.width();

			int16_t x = PAGES_POS.x() - width / 2;

			for (size_t i = 0; i < page_texts.size(); i++)
			{
				int16_t text_width = page_texts[i].width();

				if (i % 2 == 0 && cursor.x() >= x && cursor.x() < x + text_width)
				{
					if (clicked && i / 2 != page)
						change_page(i / 2);

					return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
				}

				x += text_width;
			}
		}

		// The Item Inventory's tabs.
		Point<int16_t> in_tabs = cursor - ITEM_TABS_POS;

		if (in_tabs.y() >= 0 && in_tabs.y() < ITEM_TAB_SIZE.y() && in_tabs.x() >= 0 && in_tabs.x() < ITEM_TAB_STEP * ITEM_TABS)
		{
			InventoryType::Id clicked_tab = static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + in_tabs.x() / ITEM_TAB_STEP);

			// A tab strip (CCtrlTab, FUN_004b0986): the selected tab sounds too.
			if (clicked)
				Sound(Sound::Name::TAB).play();

			if (clicked && clicked_tab != item_tab)
			{
				item_tab = clicked_tab;
				item_row = 0;
				update_item_slider();
			}

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICashShop::update_tooltip(Point<int16_t> cursor, Point<int16_t> cursorpos)
	{
		// A commodity on the page (FUN_004b7297) has its tooltip built once where the cursor met its icon. Off
		// the icons it stays up until the cursor leaves the list.
		Point<int16_t> in_list = cursor - LIST_WINDOW;

		if (tab != MAIN && in_list.x() >= 0 && in_list.y() >= 0 && in_list.x() < LIST_SIZE.x()
			&& in_list.y() < LIST_SIZE.y())
		{
			for (size_t i = 0; i < PAGE_SIZE && page * PAGE_SIZE + i < listed.size(); i++)
			{
				Point<int16_t> in_icon = cursor - LIST_POS - LIST_ICON
					- Point<int16_t>(CELL_STEP.x() * (i % LIST_COLUMNS), CELL_STEP.y() * (i / LIST_COLUMNS));

				if (in_icon.x() < 0 || in_icon.y() < 0 || in_icon.x() >= LIST_ICON_SIZE || in_icon.y() >= LIST_ICON_SIZE)
					continue;

				if (hovered == static_cast<int16_t>(i))
					return;

				// FUN_004baf1a: only cash items, packages and the 911, 5430 and 8xxxxxxx specials.
				const Commodity& commodity = listed[page * PAGE_SIZE + i];
				int32_t item_id = commodity.item_id;
				bool special = item_id / 10000 == 910 || item_id / 10000 == 911 || item_id / 1000 == 5430
					|| commodity.sn / 10000000 == 8;

				if (!special && !ItemData::get(item_id).is_cashitem())
				{
					clear_tooltip();
					return;
				}

				tooltip_pos = cursorpos + TOOLTIP_RIGHT;

				if (item_id / 1000000 == 1)
				{
					eqtooltip.set_commodity(item_id, commodity.period);
					tooltip = &eqtooltip;
				}
				else if (item_id / 10000 == 910)
				{
					packagetooltip.set_package(item_id, catalog);
					tooltip = &packagetooltip;
					tooltip_pos = cursorpos + PACKAGE_TOOLTIP;
				}
				else
				{
					ittooltip.set_commodity(item_id, commodity.period);
					tooltip = &ittooltip;
				}
				hovered = static_cast<int16_t>(i);
				hovered_cash = 0;

				return;
			}

			hovered = -1;

			return;
		}

		// The grids are rebuilt on every move (FUN_004b0764, FUN_004b180c), so they follow the cursor.
		auto icon_cell = [&](Point<int16_t> grid, size_t columns, size_t rows) -> int16_t
		{
			Point<int16_t> in_grid = cursor - grid;
			bool on_icon = in_grid.x() % GRID_STEP < ICON_SIZE && in_grid.y() % GRID_STEP < ICON_SIZE;

			return on_icon ? grid_cell(cursor, grid, columns, rows) : -1;
		};

		int16_t cash_cell = icon_cell(CASH_GRID_POS, CASH_COLUMNS, CASH_ROWS);
		size_t cash_index = cash_row * CASH_COLUMNS + cash_cell;

		if (cash_cell >= 0 && cash_index < cash_items.size())
		{
			const CashItem& item = cash_items[cash_index];

			if (!tooltip || hovered_cash != item.cash_id)
			{
				if (item.item_id / 1000000 == 1)
				{
					eqtooltip.set_cash_item(item.item_id, item.expiration, item.gift_from);
					tooltip = &eqtooltip;
				}
				else
				{
					ittooltip.set_cash_item(item.item_id, item.expiration, item.gift_from);
					tooltip = &ittooltip;
				}

				hovered = -1;
				hovered_cash = item.cash_id;
			}

			tooltip_pos = cursorpos + TOOLTIP_RIGHT;

			return;
		}

		// Only cash items in the Item Inventory, with the tooltip they have in game.
		int16_t item_cell = icon_cell(ITEM_GRID_POS, ITEM_COLUMNS, ITEM_ROWS);
		const Inventory& inventory = Stage::get().get_player().get_inventory();
		int16_t slot = static_cast<int16_t>(item_row * ITEM_COLUMNS + item_cell + 1);
		int32_t item_id = item_cell >= 0 ? inventory.get_item_id(item_tab, slot) : 0;

		if (item_id && ItemData::get(item_id).is_cashitem())
		{
			if (item_tab == InventoryType::Id::EQUIP)
			{
				eqtooltip.set_equip(Tooltip::Parent::ITEMINVENTORY, slot);
				tooltip = &eqtooltip;
			}
			else
			{
				auto item = inventory.get_item(item_tab, slot);
				auto pet = item_tab == InventoryType::Id::CASH ? inventory.get_pet(inventory.get_cash_id(item_tab, slot))
															   : Optional<const Pet>();

				if (item)
					ittooltip.set_item(item_id, item->get_expiration(), item->get_flags(), true);
				else if (pet)
					ittooltip.set_item(item_id, pet->get_date_dead(), 0, true, pet->get_remain_life());
				else
					ittooltip.set_item(item_id);

				tooltip = &ittooltip;
			}

			tooltip_pos = cursorpos + TOOLTIP_BELOW;
			hovered = -1;
			hovered_cash = 0;

			return;
		}

		clear_tooltip();
	}

	void UICashShop::clear_tooltip()
	{
		tooltip = nullptr;
		hovered = -1;
		hovered_cash = 0;
	}

	void UICashShop::remove_cursor()
	{
		UIElement::remove_cursor();

		clear_tooltip();
	}

	void UICashShop::doubleclick(Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor = cursorpos - position;
		const Inventory& inventory = Stage::get().get_player().get_inventory();

		int16_t cash_cell = grid_cell(cursor, CASH_GRID_POS, CASH_COLUMNS, CASH_ROWS);
		size_t index = cash_row * CASH_COLUMNS + cash_cell;

		if (cash_cell >= 0 && index < cash_items.size())
		{
			const CashItem& item = cash_items[index];
			InventoryType::Id type = InventoryType::by_item_id(item.item_id);

			// The server picks another slot, or refuses, if this one is taken.
			if (request())
				TakeOutCashItemPacket(item.cash_id, type, inventory.find_free_slot(type)).dispatch();
			return;
		}

		int16_t item_cell = grid_cell(cursor, ITEM_GRID_POS, ITEM_COLUMNS, ITEM_ROWS);

		if (item_cell >= 0)
		{
			int16_t slot = static_cast<int16_t>(item_row * ITEM_COLUMNS + item_cell + 1);

			if (int64_t cash_id = inventory.get_cash_id(item_tab, slot); cash_id && request())
				PutBackCashItemPacket(cash_id, item_tab).dispatch();

			return;
		}

		// A Best Item buys on a double-click, after the level gates its BUY button lacks
		// (FUN_004bb884).
		Point<int16_t> in_best = cursor - BEST_WINDOW;
		bool on_best = in_best.x() >= 0 && in_best.x() < BEST_WIDTH && in_best.y() >= 0 && in_best.y() % BEST_STEP < BEST_STEP - 1;
		size_t best = on_best ? in_best.y() / BEST_STEP : best_items.size();

		if (best < best_items.size() && !pending)
		{
			const Commodity& commodity = best_items[best].commodity;
			int16_t level = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::LEVEL);

			if (const char* refusal = best_item_refusal(commodity.item_id, level))
				UI::get().emplace<UIOk>(refusal, [](bool) {});
			else
				UICashBuy::open(commodity, credit, points, prepaid);
		}
	}

	bool UICashShop::request()
	{
		if (pending)
			return false;

		pending = true;

		return true;
	}

	void UICashShop::answered()
	{
		pending = false;
	}

	void UICashShop::set_name_change(int32_t sn)
	{
		name_change_sn = sn;
	}

	int32_t UICashShop::get_name_change() const
	{
		return name_change_sn;
	}

	int16_t UICashShop::grid_cell(Point<int16_t> cursor, Point<int16_t> grid, size_t columns, size_t rows) const
	{
		Point<int16_t> in_grid = cursor - grid;

		if (in_grid.x() < 0 || in_grid.y() < 0 || in_grid.x() >= GRID_STEP * columns || in_grid.y() >= GRID_STEP * rows)
			return -1;

		return static_cast<int16_t>(in_grid.y() / GRID_STEP * columns + in_grid.x() / GRID_STEP);
	}

	UIElement::Type UICashShop::get_type() const
	{
		return TYPE;
	}

	void UICashShop::exit_cashshop()
	{
		UI& ui = UI::get();
		ui.change_state(UI::State::GAME);

		Stage& stage = Stage::get();
		Player& player = stage.get_player();

		PlayerLoginPacket(player.get_oid()).dispatch();

		int32_t mapid = player.get_stats().get_mapid();
		uint8_t portalid = player.get_stats().get_portal();

		stage.load(mapid, portalid);
		stage.transfer_player();

		ui.enable();
		Timer::get().start();
		GraphicsGL::get().unlock();
	}

	void UICashShop::leave()
	{
		Window::get().fadeout(
			[]()
			{
				GraphicsGL::get().clear();
				ChangeMapPacket().dispatch();
			}
		);

		GraphicsGL::get().lock();
		Stage::get().clear();
		Timer::get().start();
	}

	void UICashShop::confirm_exit()
	{
		UI::get().emplace<UIYesNo>("Are you sure you want to leave the Cash Shop?", [this](bool yes)
		{
			if (yes)
				button_pressed(Buttons::BT_EXIT);
		});
	}

	void UICashShop::set_account(const std::string& account, const std::vector<int32_t>& best)
	{
		account_label.change_text(account);
		best_items.clear();

		for (const Commodity& commodity : find(commodities, best))
		{
			if (best_items.size() == BEST_ITEMS)
				break;

			if (!ItemData::get(commodity.item_id).is_valid())
				continue;

			Best entry = {
				commodity,
				Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(commodity.item_id).get_name()),
				Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, price_text(commodity))
			};

			string_format::format_with_ellipsis(entry.name, 72);
			string_format::format_with_ellipsis(entry.price, 78);
			best_items.push_back(entry);
		}
	}

	void UICashShop::set_balance(int32_t nx_credit, int32_t maple_points, int32_t nx_prepaid)
	{
		credit = nx_credit;
		points = maple_points;
		prepaid = nx_prepaid;

		// The art labels the rows NX Credit, NX Prepaid, Maple Point.
		int32_t balances[3] = { credit, prepaid, points };

		for (size_t i = 0; i < 3; i++)
		{
			std::string text = std::to_string(balances[i]);
			string_format::split_number(text);
			balance_labels[i].change_text(text);
		}
	}

	void UICashShop::set_cash_items(const std::vector<CashItem>& items)
	{
		cash_items = items;
		cash_row = 0;
		update_cash_slider();
	}

	void UICashShop::set_storage_slots(int16_t slots)
	{
		storage_slots = slots;
	}

	int16_t UICashShop::get_storage_slots() const
	{
		return storage_slots;
	}

	void UICashShop::refresh_slots()
	{
		update_item_slider();
	}

	void UICashShop::add_cash_items(const std::vector<CashItem>& items)
	{
		cash_items.insert(cash_items.end(), items.begin(), items.end());
		update_cash_slider();
	}

	void UICashShop::remove_cash_item(int64_t cash_id)
	{
		cash_items.erase(
			std::remove_if(cash_items.begin(), cash_items.end(), [cash_id](const CashItem& item) { return item.cash_id == cash_id; }),
			cash_items.end()
		);

		if (hovered_cash == cash_id)
			clear_tooltip();

		size_t rows = (cash_items.size() + CASH_COLUMNS - 1) / CASH_COLUMNS;

		if (cash_row + CASH_ROWS > rows)
			cash_row = rows > CASH_ROWS ? rows - CASH_ROWS : 0;

		update_cash_slider();
	}

	void UICashShop::set_wish_list(const std::vector<int32_t>& sns)
	{
		wish_list = sns;

		if (tab == WISH_LIST)
			update_list();
	}

	bool UICashShop::added_wish() const
	{
		return wish_added;
	}

	void UICashShop::reserve(int32_t sn)
	{
		if (const char* refusal = wish_refusal(wish_list, sn))
		{
			UI::get().emplace<UIOk>(refusal, [](bool) {});
		}
		else if (request())
		{
			wish_added = true;
			SetWishListPacket(wish(wish_list, sn)).dispatch();
		}
	}

	void UICashShop::change_tab(Tab new_tab)
	{
		tab = new_tab;
		subtab = 0;
		page = 0;

		update_list();
	}

	void UICashShop::change_subtab(size_t new_subtab)
	{
		subtab = new_subtab;
		page = 0;

		update_list();
	}

	void UICashShop::change_page(size_t new_page)
	{
		page = new_page;

		update_page();
	}

	void UICashShop::update_list()
	{
		subtab_texts.clear();

		const std::vector<std::string>& names = subtab_names[tab];

		for (size_t i = 0; i < names.size(); i++)
		{
			std::string text = i + 1 < names.size() ? names[i] + " |" : names[i];
			subtab_texts.emplace_back(subtab_font(i == subtab), Text::Alignment::LEFT, subtab_color(tab), text);
		}

		if (tab == WISH_LIST)
			listed = find(commodities, wish_list);
		else if (tab == MAIN)
			listed.clear();
		else
			listed = list(commodities, category(tab), static_cast<int32_t>(subtab), female);

		// Skip commodities whose item this client's data doesn't have.
		listed.erase(
			std::remove_if(listed.begin(), listed.end(), [](const Commodity& commodity) { return !ItemData::get(commodity.item_id).is_valid(); }),
			listed.end()
		);

		update_page();
	}

	void UICashShop::update_page()
	{
		// Another page, sub-tab, tab or wish list.
		clear_tooltip();

		page_texts.clear();

		size_t pages = page_count(listed.size());

		for (size_t i = 0; i < pages; i++)
		{
			page_texts.emplace_back(i == page ? Text::Font::A11B : Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, std::to_string(i + 1));

			if (i + 1 < pages)
				page_texts.emplace_back(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, "|");
		}

		for (size_t i = 0; i < PAGE_SIZE; i++)
		{
			size_t index = page * PAGE_SIZE + i;
			bool shown = tab != MAIN && index < listed.size();
			uint16_t first = Buttons::BT_LIST + 3 * i;

			buttons[first]->set_active(shown);
			buttons[first + 1]->set_active(shown);
			buttons[first + 2]->set_active(shown && tab != WISH_LIST);
			buttons[Buttons::BT_LIST + 3 * PAGE_SIZE + i]->set_active(shown && tab == WISH_LIST);

			if (!shown)
				continue;

			const Commodity& commodity = listed[index];

			cells[i].name.change_text(ItemData::get(commodity.item_id).get_name());
			cells[i].price.change_text(price_text(commodity));

			string_format::format_with_ellipsis(cells[i].name, 114);
		}
	}

	void UICashShop::update_cash_slider()
	{
		int16_t rows = static_cast<int16_t>((cash_items.size() + CASH_COLUMNS - 1) / CASH_COLUMNS);

		cash_slider.setrows(static_cast<int16_t>(cash_row), CASH_ROWS, std::max<int16_t>(rows, CASH_ROWS));
	}

	void UICashShop::update_item_slider()
	{
		const Inventory& inventory = Stage::get().get_player().get_inventory();
		int16_t rows = static_cast<int16_t>((inventory.get_slotmax(item_tab) + ITEM_COLUMNS - 1) / ITEM_COLUMNS);

		item_slider.setrows(static_cast<int16_t>(item_row), ITEM_ROWS, std::max<int16_t>(rows, ITEM_ROWS));
	}
}
