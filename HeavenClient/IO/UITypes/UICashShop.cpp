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
#include "../../Character/Look/CashWeapon.h"
#include "../../Data/EquipData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Misc.h"
#include "../../Util/Randomizer.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/LoginPackets.h"

#include "Timer.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <utility>

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
		// Where in a cell the shop puts the pointer to show an item.
		const Point<int16_t> CELL_POINTER = Point<int16_t>(34, 34);
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

		// The mannequin's ring slots, in the order they fill.
		constexpr EquipSlot::Id RINGS[4] = { EquipSlot::Id::RING1, EquipSlot::Id::RING2, EquipSlot::Id::RING3, EquipSlot::Id::RING4 };

		// The mannequin is the player's look, on foot and as themselves.
		CharLook mannequin()
		{
			CharLook look = Stage::get().get_player().get_look();
			look.set_vehicle(0);
			look.set_morph(0);
			look.set_ghost(0);

			return look;
		}

		std::string price_text(const Commodity& commodity)
		{
			// StringPool 0x24c, unsplit.
			if (for_mesos(commodity.sn))
				return std::to_string(commodity.price) + " mesos";

			std::string price = std::to_string(commodity.price);
			string_format::split_number(price);
			price += " NX Cash";

			if (commodity.count > 1)
				price += "(" + std::to_string(commodity.count) + ")";

			return price;
		}
	}

	UICashShop::UICashShop() : tab(EVENT_TAB), subtab(0), page(0), preview_index(0), female(false), credit(0), points(0), prepaid(0), pending(false), wish_added(false), name_change_sn(0), present(std::exchange(cash_catalog::pending_present(), std::nullopt)), storage_slots(0), character_slots(0), cash_row(0), item_tab(InventoryType::Id::EQUIP), item_row(0)
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
		preview_look = mannequin();

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
		item_selected = CSList["ItemIcon"];
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
				static_cast<int32_t>(item["Period"].get_integer()),
				item["OnSale"].get_integer() == 1
			});
		}

		commodities = on_sale(catalog);
		mesos_placed = scatter(commodities, []() { return Randomizer().next_int<uint32_t>(UINT32_MAX); });

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

		// Opened from the item window, at the commodity it asked for
		// (v83 469550, 4bc324), once the server's changes are in.
		opening = std::exchange(opening_sn(), 0);
	}

	void UICashShop::draw(float inter) const
	{
		// The preview shows through the background's transparent hole.
		preview_backgrounds[preview_index].draw(position + PREVIEW_POS);
		background.draw(position);
		preview_frame.draw(position + PREVIEW_FRAME_POS);

		const Player& player = Stage::get().get_player();
		preview_look.draw(position + AVATAR_POS, false, Stance::Id::STAND1, Expression::Id::DEFAULT);

		// A pet tried on stands 80 to the right (7036bc).
		if (tried_pet.item_id)
			preview_pet.draw_standing(position + AVATAR_POS + Point<int16_t>(80, 0), inter);

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

				// The selected row's frame, not while a request is out (4b7b98).
				if (static_cast<int16_t>(i) == selected_row && !pending)
					item_selected.draw(cell + Point<int16_t>(4, 4));

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

		if (int32_t sn = std::exchange(opening, 0))
		{
			auto commodity = std::find_if(commodities.begin(), commodities.end(), [sn](const Commodity& c) { return c.sn == sn; });

			if (commodity != commodities.end())
				show(*commodity);
		}

		for (Animation& label : labels)
			label.update();

		if (tried_pet.item_id)
			preview_pet.update_standing();

		// The keys moved the selection: its tooltip shows as if the cursor
		// were on the lower part of its icon (4b7a91).
		if (std::exchange(show_selected, false) && selected_row >= 0 && selected_row < rows_shown())
		{
			Point<int16_t> cell = LIST_POS
				+ Point<int16_t>(CELL_STEP.x() * (selected_row % LIST_COLUMNS), CELL_STEP.y() * (selected_row / LIST_COLUMNS));

			show_commodity_tooltip(selected_row, position + cell + Point<int16_t>(55, 55));
		}
	}

	bool UICashShop::list_key(int32_t action, bool pressed, bool escape)
	{
		int16_t count = rows_shown();

		if (!pressed || pending || count == 0)
			return false;

		if (focused_button < 0)
		{
			int16_t row = std::max<int16_t>(selected_row, 0);

			switch (action)
			{
			case KeyAction::Id::RETURN:
				// Try the row on, with the key focus on its BUY.
				if (selected_row >= 0 && selected_row < count)
				{
					focused_button = 0;
					focus_button(selected_row, 0);
					try_on(listed[page * PAGE_SIZE + selected_row]);
				}

				return true;
			case KeyAction::Id::LEFT:
				row = row % 2 == 0 ? row + 1 : row - 1;

				if (row >= count)
					row = count - 1;

				break;
			case KeyAction::Id::RIGHT:
				row = selected_row;

				if (row == count - 1)
					row = row / 2 * 2;
				else if (row % 2 == 1)
					row--;
				else
					row++;

				break;
			case KeyAction::Id::UP:
				if (row > 1)
				{
					row -= 2;
				}
				else
				{
					row += count / 2 * 2;

					if (row >= count)
						row -= 2;
				}

				break;
			case KeyAction::Id::DOWN:
				row = count - row < 3 ? row % 2 : row + 2;
				break;
			default:
				return false;
			}

			selected_row = row;
			show_selected = true;

			return true;
		}

		if (escape)
		{
			focused_button = -1;
			focus_button(-1, -1);
			show_selected = true;

			return true;
		}

		switch (action)
		{
		case KeyAction::Id::RETURN:
			if (selected_row >= 0 && selected_row < count)
			{
				uint16_t id = list_button(selected_row, focused_button);
				Button& button = *buttons[id];

				if (button.is_active() && button.get_state() != Button::State::DISABLED)
				{
					Sound(Sound::Name::BUTTONCLICK).play();
					button.set_state(button_pressed(id));
				}
			}

			return true;
		case KeyAction::Id::LEFT:
			focused_button = focused_button == 0 ? 2 : focused_button - 1;
			focus_button(selected_row, focused_button);

			return true;
		case KeyAction::Id::RIGHT:
			focused_button = focused_button == 2 ? 0 : focused_button + 1;
			focus_button(selected_row, focused_button);

			return true;
		default:
			return false;
		}
	}

	int16_t UICashShop::rows_shown() const
	{
		size_t first = page * PAGE_SIZE;

		if (tab == MAIN || first >= listed.size())
			return 0;

		return static_cast<int16_t>(std::min(PAGE_SIZE, listed.size() - first));
	}

	uint16_t UICashShop::list_button(int16_t row, int16_t n) const
	{
		if (n == 2 && tab == WISH_LIST)
			return static_cast<uint16_t>(Buttons::BT_LIST + 3 * PAGE_SIZE + row);

		return static_cast<uint16_t>(Buttons::BT_LIST + 3 * row + n);
	}

	void UICashShop::focus_button(int16_t row, int16_t n)
	{
		for (size_t i = 0; i < 4 * PAGE_SIZE; i++)
			buttons[Buttons::BT_LIST + i]->set_focused(false);

		if (row >= 0 && n >= 0)
			buttons[list_button(row, n)]->set_focused(true);
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
		case Buttons::BT_BUY_AVATAR:
			buy_avatar();

			return Button::State::NORMAL;
		case Buttons::BT_DEFAULT_AVATAR:
			default_avatar();

			return Button::State::NORMAL;
		case Buttons::BT_TAKEOFF_AVATAR:
			take_off_avatar();

			return Button::State::NORMAL;
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
				else if (list_button % 3 == 0 && for_mesos(commodity.sn))
					buy_with_mesos(commodity);
				// BUY or GIFT, a ring of a pair is for two (v83 472fb6).
				else if (list_button % 3 == 0 || (list_button % 3 == 1 && marriage::is_pair_ring(commodity.item_id)))
					UICashBuy::open(commodity, credit, points, prepaid);
				else if (list_button % 3 == 1)
					gift(commodity);
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

				show_commodity_tooltip(i, cursorpos);

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

	void UICashShop::show_commodity_tooltip(size_t i, Point<int16_t> anchor)
	{
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

		tooltip_pos = anchor + TOOLTIP_RIGHT;

		if (item_id / 1000000 == 1)
		{
			eqtooltip.set_commodity(item_id, commodity.period);
			tooltip = &eqtooltip;
		}
		else if (item_id / 10000 == 910)
		{
			packagetooltip.set_package(item_id, catalog);
			tooltip = &packagetooltip;
			tooltip_pos = anchor + PACKAGE_TOOLTIP;
		}
		else if (is_character_slot(item_id))
		{
			// StringPool 0x245 (FUN_008f19a8).
			ittooltip.set_commodity(item_id, commodity.period, "Add a character slot (Current : " +
				std::to_string(character_slots) + " slots)\\nYou may have up to " +
				std::to_string(MAX_CHARACTER_SLOTS) + " slots.");
			tooltip = &ittooltip;
		}
		else
		{
			ittooltip.set_commodity(item_id, commodity.period);
			tooltip = &ittooltip;
		}

		hovered = static_cast<int16_t>(i);
		hovered_cash = 0;
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

		int16_t cash_cell = grid_cell(cursor, CASH_GRID_POS, CASH_COLUMNS, CASH_ROWS);
		size_t index = cash_row * CASH_COLUMNS + cash_cell;

		if (cash_cell >= 0 && index < cash_items.size())
		{
			take_out(cash_items[index]);

			return;
		}

		int16_t item_cell = grid_cell(cursor, ITEM_GRID_POS, ITEM_COLUMNS, ITEM_ROWS);

		if (item_cell >= 0)
		{
			put_back(static_cast<int16_t>(item_row * ITEM_COLUMNS + item_cell + 1));

			return;
		}

		// A commodity's icon tries it on.
		for (size_t i = 0; tab != MAIN && i < PAGE_SIZE && page * PAGE_SIZE + i < listed.size(); i++)
		{
			Point<int16_t> in_icon = cursor - LIST_POS - LIST_ICON
				- Point<int16_t>(CELL_STEP.x() * (i % LIST_COLUMNS), CELL_STEP.y() * (i / LIST_COLUMNS));

			if (in_icon.x() >= 0 && in_icon.y() >= 0 && in_icon.x() < LIST_ICON_SIZE && in_icon.y() < LIST_ICON_SIZE)
			{
				// And selects its row (4b70ce).
				clear_tooltip();
				try_on(listed[page * PAGE_SIZE + i]);
				selected_row = static_cast<int16_t>(i);

				return;
			}
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
		// Buy Avatar's purchases count as one request, from the first to
		// the last.
		if (pending || buying_avatar())
			return false;

		pending = true;

		return true;
	}

	void UICashShop::answered()
	{
		pending = false;
	}

	void UICashShop::gift_looked_up(int32_t birthday, int32_t sn, const std::string& to, const std::string& message,
									const cash_gift::Receiver& receiver)
	{
		auto by_sn = [this](int32_t serial)
		{
			return std::find_if(catalog.begin(), catalog.end(), [serial](const Commodity& c) { return c.sn == serial; });
		};
		auto commodity = by_sn(sn);

		if (birthday == 0 || to.empty() || message.empty() || commodity == catalog.end() || commodity->price == 0)
			return;

		// StringPool 0x217. v83 checks NX Credit here, where the button
		// checked NX Prepaid.
		if (credit < commodity->price)
		{
			UI::get().emplace<UIOk>("You don't have enough cash.", [](bool) {});

			return;
		}

		bool package = commodity->item_id / 10000 == 910;
		std::vector<cash_gift::Member> members;

		if (package)
		{
			for (nl::node member : nl::nx::etc["CashPackage.img"][std::to_string(commodity->item_id)]["SN"])
			{
				auto content = by_sn(static_cast<int32_t>(member.get_integer()));

				if (content != catalog.end())
					members.push_back({ content->item_id, content->period });
			}
		}
		else
		{
			members.push_back({ commodity->item_id, commodity->period });
		}

		cash_gift::Lookup lookup;
		lookup.equip = [](int32_t item_id) -> std::optional<cash_gift::Equip>
		{
			const EquipData& data = EquipData::get(item_id);

			if (!data.is_valid())
				return std::nullopt;

			return cash_gift::Equip{ data.get_reqstat(MapleStat::Id::JOB), data.get_reqstat(MapleStat::Id::LEVEL),
									 data.get_reqstat(MapleStat::Id::FAME) };
		};
		lookup.covers = [](int32_t sticker, int32_t weapon)
		{
			int32_t type = cash_weapon::type(weapon);
			nl::node art = nl::nx::character["Weapon"]["0" + std::to_string(sticker) + ".img"];

			return type != 0 && art[std::to_string(type)].size() > 0;
		};

		// StringPool 0x200, after the questions. v83 then offers only NX
		// Credit to pay with.
		std::string confirm = ItemData::get(commodity->item_id).get_name() + " (" + std::to_string(commodity->price) +
							  " NX)\\nWill you buy this item as a gift for\\n'" + to + "'?";
		auto send = [birthday, sn, to, message, mode = static_cast<uint8_t>(package ? 0x1F : 4)]()
		{
			if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
				GiftCashItemPacket(birthday, sn, to, message, mode).dispatch();
		};
		auto questions = std::make_shared<std::vector<cash_gift::Question>>(cash_gift::questions(package, members, receiver, lookup));
		auto ask = std::make_shared<std::function<void(size_t)>>();

		*ask = [questions, confirm, send, weak = std::weak_ptr<std::function<void(size_t)>>(ask)](size_t i)
		{
			if (i == questions->size())
			{
				UI::get().emplace<UIYesNo>(confirm, [send](bool yes) { if (yes) send(); });

				return;
			}

			const cash_gift::Question& question = (*questions)[i];

			if (!question.ask)
			{
				UI::get().emplace<UIOk>(question.text, [](bool) {});

				return;
			}

			UI::get().emplace<UIYesNo>(question.text, [next = weak.lock(), i](bool yes)
			{
				if (yes && next)
					(*next)(i + 1);
			});
		};

		(*ask)(0);
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

	bool UICashShop::buying_avatar() const
	{
		return avatar_payment != 0;
	}

	void UICashShop::avatar_item_bought()
	{
		avatar_bought.push_back(avatar_left);
	}

	void UICashShop::try_on(const Commodity& commodity)
	{
		if (commodity.item_id / 10000 != 910)
		{
			try_on_item(commodity.item_id, commodity.sn, false);

			return;
		}

		std::vector<Commodity> contents;

		for (nl::node sn : nl::nx::etc["CashPackage.img"][std::to_string(commodity.item_id)]["SN"])
		{
			int32_t serial = static_cast<int32_t>(sn.get_integer());
			auto content = std::find_if(catalog.begin(), catalog.end(), [serial](const Commodity& c) { return c.sn == serial; });

			if (content != catalog.end())
				contents.push_back(*content);
		}

		// A package for either gender is its first gendered equip's (4bb716).
		Commodity gendered = commodity;

		for (const Commodity& content : contents)
		{
			if (gendered.gender == 2 && content.item_id / 1000000 == 1 && content.item_id / 1000 % 10 < 2)
				gendered.gender = static_cast<int8_t>(content.item_id / 1000 % 10);
		}

		if (!for_gender(gendered, female))
			return;

		// Each item that goes on marks the package (4bb5ef), so the next
		// disables Buy Avatar.
		for (const Commodity& content : contents)
			if (try_on_item(content.item_id, content.sn, true))
				package_on = true;
	}

	bool UICashShop::try_on_item(int32_t item_id, int32_t sn, bool packaged)
	{
		// A pet stands beside the mannequin; the same one again goes away,
		// unless a package puts it on (4acd22).
		if (item_id / 10000 == 500)
		{
			if (tried_pet.item_id != item_id)
			{
				tried_pet = { item_id, sn };
				preview_pet = PetLook(PetEntry{ item_id, "", 0, Point<int16_t>(), 0, 0 }, false);
			}
			else if (!packaged)
			{
				tried_pet = { 0, 0 };
				preview_pet = PetLook();
			}

			return true;
		}

		if (!fits_mannequin(item_id, female))
			return false;

		// A cash weapon is worn over the weapon, in the weapon's slot.
		bool cash_weapon = item_id / 10000 == 170;
		EquipSlot::Id slot = cash_weapon ? EquipSlot::Id::WEAPON : EquipData::get(item_id).get_eqslot();

		// The slots 4ac676 allows; a mount or saddle only in a package
		// (4aceb6).
		bool mount = slot == EquipSlot::Id::TAMEDMOB || slot == EquipSlot::Id::SADDLE;

		if (slot < EquipSlot::Id::HAT || (slot > EquipSlot::Id::RING4 && !mount) || (mount && !packaged))
			return false;

		// v83 disables Buy Avatar for anything tried on over a package.
		if (package_on)
			buttons[Buttons::BT_BUY_AVATAR]->set_state(Button::State::DISABLED);

		// The item already in its slot comes off, unless a package puts it
		// on.
		if (slot == EquipSlot::Id::RING1)
		{
			for (EquipSlot::Id ring : RINGS)
			{
				if (preview_look.get_ring(ring) == item_id && !packaged)
				{
					take_off(ring);

					return true;
				}
			}

			// The first free ring slot, or the first.
			for (EquipSlot::Id ring : RINGS)
			{
				if (preview_look.get_ring(ring) == 0)
				{
					slot = ring;
					break;
				}
			}

			preview_look.set_ring(slot, item_id);
			tried[slot] = { item_id, sn };

			return true;
		}

		int32_t worn = cash_weapon ? preview_look.get_cash_weapon() : preview_look.get_equips().get_equip(slot);

		if (worn == item_id && !packaged)
		{
			if (cash_weapon)
				preview_look.set_cash_weapon(0);
			else
				preview_look.remove_equip(slot);

			tried.erase(slot);

			return true;
		}

		if (cash_weapon)
		{
			tried[slot] = { item_id, sn };
			preview_look.set_cash_weapon(item_id);

			return true;
		}

		// An overall takes the pants off, a coat or pants an overall, a
		// shield a two-handed weapon, and a weapon the cash weapon.
		int32_t kind = item_id / 10000;

		if (kind == 105)
			take_off(EquipSlot::Id::BOTTOM);
		else if ((kind == 104 || kind == 106) && preview_look.get_equips().has_overall())
			take_off(EquipSlot::Id::TOP);
		else if (slot == EquipSlot::Id::SHIELD && preview_look.get_equips().is_twohanded())
			take_off(EquipSlot::Id::WEAPON);
		else if (slot == EquipSlot::Id::WEAPON)
			preview_look.set_cash_weapon(0);

		// After the replacements, which may clear this slot's entry.
		tried[slot] = { item_id, sn };
		preview_look.add_equip(item_id);

		// A two-handed weapon takes the shield off.
		if (slot == EquipSlot::Id::WEAPON && preview_look.get_equips().is_twohanded())
			take_off(EquipSlot::Id::SHIELD);

		return true;
	}

	void UICashShop::take_off(int16_t slot)
	{
		preview_look.remove_equip(static_cast<EquipSlot::Id>(slot));
		preview_look.set_ring(slot, 0);
		tried.erase(slot);
	}

	void UICashShop::default_avatar()
	{
		preview_look = mannequin();
		tried.clear();
		tried_pet = { 0, 0 };
		preview_pet = PetLook();
		package_on = false;
		buttons[Buttons::BT_BUY_AVATAR]->set_state(Button::State::NORMAL);
	}

	void UICashShop::take_off_avatar()
	{
		for (int16_t slot = EquipSlot::Id::HAT; slot < EquipSlot::Id::LENGTH; slot++)
			take_off(slot);

		preview_look.set_cash_weapon(0);
		tried.clear();
		tried_pet = { 0, 0 };
		preview_pet = PetLook();
		package_on = false;
		buttons[Buttons::BT_BUY_AVATAR]->set_state(Button::State::NORMAL);
	}

	void UICashShop::buy_avatar()
	{
		if (pending || buying_avatar())
			return;

		if (package_on)
		{
			UI::get().emplace<UIOk>("The option is not available for that package.", [](bool) {});

			return;
		}

		// What was tried on, but not what the character wears, a ring in
		// any of its slots (4ac852).
		const CharLook& own = Stage::get().get_player().get_look();
		std::vector<int32_t> sns;

		for (const auto& [slot, item] : tried)
		{
			bool worn = false;

			if (slot == EquipSlot::Id::WEAPON && item.item_id / 10000 == 170)
				worn = own.get_cash_weapon() == item.item_id;
			else if (std::find(std::begin(RINGS), std::end(RINGS), slot) != std::end(RINGS))
				worn = std::any_of(std::begin(RINGS), std::end(RINGS), [&](EquipSlot::Id ring) { return own.get_ring(ring) == item.item_id; });
			else
				worn = own.get_equips().get_equip(static_cast<EquipSlot::Id>(slot)) == item.item_id;

			if (item.sn != 0 && !worn)
				sns.push_back(item.sn);
		}

		// The pet, unless it's the kind of the character's first pet out
		// (4acda4).
		auto own_pet = Stage::get().get_player().get_pet(0);

		if (tried_pet.sn != 0 && !(own_pet && own_pet->get_itemid() == tried_pet.item_id))
			sns.push_back(tried_pet.sn);

		if (sns.empty())
			return;

		// The first item the character has: a free one can't be bought
		// again (0xb94), another asks first (0x1f2), and v83 checks the rest
		// no more, nor whether they're free.
		int32_t total = 0;
		bool any_free = false;
		bool asked = false;

		for (int32_t sn : sns)
		{
			auto commodity = std::find_if(catalog.begin(), catalog.end(), [sn](const Commodity& c) { return c.sn == sn; });

			if (commodity == catalog.end())
				continue;

			total += commodity->price;

			if (asked)
				continue;

			any_free = any_free || commodity->price == 0;

			if (!owns(commodity->item_id, true))
				continue;

			if (commodity->price == 0)
			{
				UI::get().emplace<UIOk>("You may not purchase the item\\nsince you already have it.", [](bool) {});

				return;
			}

			asked = true;
		}

		std::array<bool, NUM_PAYMENTS> offered = avatar_payment_options(total, any_free, credit, points, prepaid);

		auto confirm = [sns, total, offered]()
		{
			if (std::none_of(offered.begin(), offered.end(), [](bool option) { return option; }))
				UI::get().emplace<UIOk>("You don't have enough cash.", [](bool) {});
			else
				UI::get().emplace<UICashBuy>(sns, total, offered);
		};

		if (asked)
			UI::get().emplace<UIYesNo>("You already have the item(s) selected. Are\\nyou sure you want to buy the item(s) again?", [confirm](bool yes) { if (yes) confirm(); });
		else
			confirm();
	}

	void UICashShop::buy_avatar(const std::vector<int32_t>& sns, int32_t payment)
	{
		if (pending || buying_avatar() || sns.empty())
			return;

		avatar_sns = sns;
		avatar_bought.clear();
		avatar_left = sns.size();
		avatar_payment = payment;

		buy_next_avatar_item();
	}

	void UICashShop::stop_buying_avatar()
	{
		avatar_sns.clear();
		avatar_bought.clear();
		avatar_left = 0;
		avatar_payment = 0;
	}

	void UICashShop::buy_next_avatar_item()
	{
		if (avatar_left > 0)
		{
			avatar_left--;
			pending = true;
			BuyCashItemPacket(avatar_payment, avatar_sns[avatar_left]).dispatch();

			return;
		}

		// StringPool 0x211, 0x1fd, or the items bought and 0x1fe.
		std::string notice;

		if (avatar_bought.size() == avatar_sns.size())
		{
			notice = "You have successfully made the purchase.";
		}
		else if (avatar_bought.empty())
		{
			notice = "The purchase has failed.";
		}
		else
		{
			for (size_t index : avatar_bought)
			{
				int32_t sn = avatar_sns[index];
				auto commodity = std::find_if(catalog.begin(), catalog.end(), [sn](const Commodity& c) { return c.sn == sn; });

				if (commodity != catalog.end())
					notice += ItemData::get(commodity->item_id).get_name();

				notice += "\\n";
			}

			notice += "\\nYou have purchased the following items.\\n(Some items weren't purchased)";
		}

		stop_buying_avatar();

		UI::get().emplace<UIOk>(notice, [](bool) {});
	}

	void UICashShop::confirm_exit()
	{
		UI::get().emplace<UIYesNo>("Are you sure you want to leave the Cash Shop?", [this](bool yes)
		{
			if (yes)
				button_pressed(Buttons::BT_EXIT);
		});
	}

	void UICashShop::apply_changes(const std::vector<Change>& changes)
	{
		if (changes.empty())
			return;

		for (const Change& change : changes)
			apply_change(catalog, change);

		commodities = on_sale(catalog);
		mesos_placed = scatter(commodities, []() { return Randomizer().next_int<uint32_t>(UINT32_MAX); });

		update_list();
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

		// The first balance after entering for a present offers it, once
		// (v83 478f81).
		if (present)
		{
			cash_catalog::Present given = *present;
			present.reset();
			give(given);
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

	void UICashShop::set_character_slots(int16_t slots)
	{
		character_slots = slots;
		// The coupon's tooltip shows the count.
		clear_tooltip();
	}

	int16_t UICashShop::get_character_slots() const
	{
		return character_slots;
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

	int32_t UICashShop::remove_cash_item(int64_t cash_id)
	{
		auto listed = std::find_if(cash_items.begin(), cash_items.end(), [cash_id](const CashItem& item) { return item.cash_id == cash_id; });
		int32_t item_id = listed == cash_items.end() ? 0 : listed->item_id;

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

		return item_id;
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
		if (const char* refusal = wish_refusal(catalog, wish_list, sn))
		{
			UI::get().emplace<UIOk>(refusal, [](bool) {});
		}
		else if (wish_sent(catalog, sn) && request())
		{
			wish_added = true;
			SetWishListPacket(wish(wish_list, sn)).dispatch();
		}
	}

	void UICashShop::buy_with_mesos(const Commodity& commodity)
	{
		if (owns(commodity.item_id, true))
		{
			UI::get().emplace<UIOk>("You may not purchase the item\\nsince you already have it.", [](bool) {});
			return;
		}

		if (Stage::get().get_player().get_inventory().get_meso() < commodity.price)
		{
			UI::get().emplace<UIOk>("You do not have enough mesos.", [](bool) {});
			return;
		}

		int32_t sn = commodity.sn;

		UI::get().emplace<UIYesNo>("Are you sure you want to buy this item?", [this, sn](bool yes)
			{
				if (yes && request())
					BuyWithMesosPacket(sn).dispatch();
			}
		);
	}

	void UICashShop::gift(const Commodity& commodity)
	{
		// Gifts are paid with NX Prepaid (StringPool 0x251 and 0xf97).
		if (commodity.price != 0 && prepaid < commodity.price)
		{
			UI::get().emplace<UIOk>("You don't have enough NX Prepaid", [](bool) {});
			return;
		}

		UI::get().emplace<UIYesNo>("Would you like to gift this item to someone?\\n[ " + ItemData::get(commodity.item_id).get_name() + " ]", [commodity](bool yes)
			{
				if (yes)
					UI::get().emplace<UICashGift>(commodity);
			}
		);
	}

	void UICashShop::give(const cash_catalog::Present& given)
	{
		std::optional<Commodity> commodity = on_sale_as(catalog, given.sn);

		// StringPool 0x24f.
		if (!commodity)
		{
			UI::get().emplace<UIOk>("The item is not on sale now.", [](bool) {});
			return;
		}

		show(*commodity);

		// The birthday code first, then NX Prepaid must cover the price,
		// then a message for the character it goes to.
		UI::get().emplace<UICashGift>(*commodity, prepaid, given.to);
	}

	void UICashShop::show(const Commodity& commodity)
	{
		int32_t cat = category(commodity.sn);

		if (cat <= MAIN || cat >= WISH_LIST)
			return;

		Tab target = static_cast<Tab>(cat);
		size_t sub = static_cast<size_t>(subcategory(commodity.sn));

		if (sub >= subtab_names[target].size())
			return;

		change_tab(target);
		change_subtab(sub);

		auto at = std::find_if(listed.begin(), listed.end(), [&commodity](const Commodity& c) { return c.sn == commodity.sn; });

		if (at == listed.end())
			return;

		size_t index = static_cast<size_t>(at - listed.begin());
		change_page(index / PAGE_SIZE);

		// The pointer goes onto the item's cell (4b95ed).
		size_t cell = index % PAGE_SIZE;
		Point<int16_t> pointer = position + LIST_POS + CELL_POINTER +
								 Point<int16_t>(CELL_STEP.x() * (cell % LIST_COLUMNS), CELL_STEP.y() * (cell / LIST_COLUMNS));
		Window::get().move_cursor(pointer.x(), pointer.y());
	}

	void UICashShop::take_out(const CashItem& item)
	{
		// A free item the character already has stays (StringPool 0xb95).
		auto commodity = std::find_if(catalog.begin(), catalog.end(), [&item](const Commodity& c) { return c.sn == item.sn; });

		if (item.sn != 0 && commodity != catalog.end() && commodity->price == 0 && owns(item.item_id, false))
		{
			UI::get().emplace<UIOk>("This item cannot be transfered because\\nyou already own this item.", [](bool) {});
			return;
		}

		const Inventory& inventory = Stage::get().get_player().get_inventory();
		InventoryType::Id type = InventoryType::by_item_id(item.item_id);

		// The server picks another slot, or refuses, if this one is taken.
		if (request())
			TakeOutCashItemPacket(item.cash_id, type, inventory.find_free_slot(type)).dispatch();
	}

	void UICashShop::put_back(int16_t slot)
	{
		const Inventory& inventory = Stage::get().get_player().get_inventory();
		int64_t cash_id = inventory.get_cash_id(item_tab, slot);

		if (!cash_id)
			return;

		// StringPool 0x9cb, for an item marked noMoveToLocker.
		if (ItemData::get(inventory.get_item_id(item_tab, slot)).is_locker_blocked())
		{
			UI::get().emplace<UIOk>("This item cannot be moved.", [](bool) {});
			return;
		}

		if (request())
			PutBackCashItemPacket(cash_id, item_tab).dispatch();
	}

	bool UICashShop::owns(int32_t item_id, bool locker) const
	{
		if (locker && std::any_of(cash_items.begin(), cash_items.end(), [item_id](const CashItem& item) { return item.item_id == item_id; }))
			return true;

		const Inventory& inventory = Stage::get().get_player().get_inventory();

		return inventory.find_item(InventoryType::by_item_id(item_id), item_id) != 0 || inventory.find_item(InventoryType::Id::EQUIPPED, item_id) != 0;
	}

	bool UICashShop::owns_package(int32_t item_id) const
	{
		for (nl::node sn : nl::nx::etc["CashPackage.img"][std::to_string(item_id)]["SN"])
		{
			int32_t serial = static_cast<int32_t>(sn.get_integer());
			auto commodity = std::find_if(catalog.begin(), catalog.end(), [serial](const Commodity& c) { return c.sn == serial; });

			if (commodity != catalog.end() && owns(commodity->item_id, true))
				return true;
		}

		return false;
	}

	void UICashShop::bought_with_mesos(int32_t item_id, int16_t slot)
	{
		item_tab = InventoryType::by_item_id(item_id);

		// v83 4b1d81 scrolls just far enough to show the slot's row.
		size_t row = slot > 0 ? (slot - 1) / ITEM_COLUMNS : 0;

		if (row < item_row)
			item_row = row;
		else if (row >= item_row + ITEM_ROWS)
			item_row = row - (ITEM_ROWS - 1);

		update_item_slider();
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
		{
			listed = list(commodities, category(tab), static_cast<int32_t>(subtab), female);
			place(listed, mesos_placed, category(tab), static_cast<int32_t>(subtab), female);
		}

		// Skip commodities whose item this client's data doesn't have.
		listed.erase(
			std::remove_if(listed.begin(), listed.end(), [](const Commodity& commodity) { return !ItemData::get(commodity.item_id).is_valid(); }),
			listed.end()
		);

		update_page();
	}

	void UICashShop::update_page()
	{
		// Another page, sub-tab, tab or wish list: the first row is
		// selected and no button has the key focus (4b988a).
		clear_tooltip();
		selected_row = 0;
		focused_button = -1;
		show_selected = false;

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

			// A commodity for mesos, or for nothing, has no GIFT or WISH (v83 4baa51).
			bool for_cash = shown && !for_mesos(listed[index].sn) && listed[index].price > 0;

			buttons[first]->set_active(shown);
			buttons[first + 1]->set_active(for_cash);
			buttons[first + 2]->set_active(for_cash && tab != WISH_LIST);
			buttons[Buttons::BT_LIST + 3 * PAGE_SIZE + i]->set_active(shown && tab == WISH_LIST);

			if (!shown)
				continue;

			const Commodity& commodity = listed[index];

			cells[i].name.change_text(ItemData::get(commodity.item_id).get_name());
			cells[i].price.change_text(price_text(commodity));

			string_format::format_with_ellipsis(cells[i].name, 114);
		}

		focus_button(-1, -1);
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
