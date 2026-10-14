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
#include "UIShop.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Audio/Audio.h"
#include "../Data/ItemData.h"

#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window, measured from its background and period footage. The
		// sell half repeats the buy half this far to the right.
		constexpr int16_t HALF = 230;
		constexpr Point<int16_t> NPC_FEET = Point<int16_t>(53, 75);
		constexpr Point<int16_t> EXIT_BUTTON = Point<int16_t>(142, 15);
		constexpr Point<int16_t> BUY_BUTTON = Point<int16_t>(142, 36);
		// The right end of the meso amount, in the field at 361-450.
		constexpr Point<int16_t> MESO = Point<int16_t>(447, 63);
		constexpr Point<int16_t> TABS = Point<int16_t>(4, 95);
		constexpr int16_t BUY_TAB_FILL = 43;
		constexpr int16_t SELL_TAB_FILL = 32;
		constexpr int16_t SELL_TABS = 5;
		// Each row is an icon box and, 36 to its right, the 162 wide entry.
		constexpr Point<int16_t> ROW = Point<int16_t>(7, 127);
		constexpr Point<int16_t> ROW_SIZE = Point<int16_t>(198, 35);
		constexpr int16_t ROW_PITCH = 40;
		constexpr int16_t ENTRY_X = 36;
		// Rech. at the right end of a row's price line.
		constexpr Point<int16_t> RECHARGE = Point<int16_t>(165, 18);
		constexpr int16_t SLIDER_X = 209;
		constexpr Range<int16_t> SLIDER_RANGE = Range<int16_t>(128, 309);

		// Within a row: the icon's top-left, the name, and the price line.
		constexpr Point<int16_t> ICON = Point<int16_t>(1, -1);
		constexpr Point<int16_t> NAME = Point<int16_t>(41, 1);
		constexpr Point<int16_t> COIN = Point<int16_t>(41, 20);
		constexpr Point<int16_t> PRICE = Point<int16_t>(55, 18);

		Point<int16_t> row_position(bool sell, int16_t row)
		{
			return ROW + Point<int16_t>(sell ? HALF : 0, ROW_PITCH * row);
		}

		void draw_entry(Point<int16_t> pos, const Texture &icon, const Text &name, const Texture &coin,
						const Text &price)
		{
			icon.draw(pos + ICON + Point<int16_t>(0, 32));
			name.draw(pos + NAME);
			coin.draw(pos + COIN);
			price.draw(pos + PRICE);
		}
	}

	UIShop::UIShop(const CharLook &in_charlook, const Inventory &in_inventory)
			: UIDragElement<PosSHOP>(), charlook(in_charlook), inventory(in_inventory)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Shop"];

		Texture bg = src["backgrnd"];
		auto bg_dimensions = bg.get_dimensions();

		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(src["BtExit"], EXIT_BUTTON);
		buttons[Buttons::BUY_ITEM] = std::make_unique<MapleButton>(src["BtBuy"], BUY_BUTTON);
		buttons[Buttons::SELL_ITEM] = std::make_unique<MapleButton>(src["BtSell"], BUY_BUTTON + Point<int16_t>(HALF, 0));

		// Only the whole list: the Must tab beside it is left out.
		buytabs = ClassicTabs(src["TabBuy"]["enabled"], src["TabBuy"]["disabled"], BUY_TAB_FILL);
		selltabs = ClassicTabs(src["TabSell"]["enabled"], src["TabSell"]["disabled"], SELL_TAB_FILL);

		for (int16_t i = 0; i < SELL_TABS; i++)
		{
			Point<int16_t> tab_pos = TABS + Point<int16_t>(HALF + selltabs.pitch() * i, 0);
			buttons[Buttons::EQUIP + i] = std::make_unique<AreaButton>(tab_pos,
																	   Point<int16_t>(selltabs.pitch(), ClassicTabs::HEIGHT));
		}

		for (int16_t i = 0; i < ROWS; i++)
		{
			buttons[Buttons::BUY0 + i] = std::make_unique<AreaButton>(row_position(false, i), ROW_SIZE);
			buttons[Buttons::SELL0 + i] = std::make_unique<AreaButton>(row_position(true, i), ROW_SIZE);
			buttons[Buttons::RECHARGE0 + i] = std::make_unique<MapleButton>(src["BtRecharge"],
																			row_position(true, i) + RECHARGE);
		}

		selection = src["select"];
		meso = src["meso"];

		mesolabel = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		buyslider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, ROWS, 1,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;
					bool above = buystate.offset + shift >= 0;
					bool below = buystate.offset + shift <= buystate.lastslot - ROWS;

					if (above && below)
						buystate.offset += shift;
				}
		);

		sellslider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X + HALF, ROWS, 1,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;
					bool above = sellstate.offset + shift >= 0;
					bool below = sellstate.offset + shift <= sellstate.lastslot - ROWS;

					if (above && below)
					{
						sellstate.offset += shift;
						update_recharge();
					}
				}
		);

		active = false;
		dimension = bg_dimensions;
		dragarea = Point<int16_t>(bg_dimensions.x(), 10);
	}

	void UIShop::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		npc.draw(DrawArgument(position + NPC_FEET, true));
		charlook.draw(position + NPC_FEET + Point<int16_t>(HALF, 0), false, Stance::Id::STAND1,
					  Expression::Id::DEFAULT);

		mesolabel.draw(position + MESO);

		buytabs.draw(position + TABS, 1, 0);
		selltabs.draw(position + TABS + Point<int16_t>(HALF, 0), SELL_TABS, tabbyinventory(sellstate.tab) - Buttons::EQUIP);

		buystate.draw(position, selection);
		sellstate.draw(position, selection);

		buyslider.draw(position);
		sellslider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIShop::update()
	{
		int64_t num_mesos = inventory.get_meso();
		std::string mesostr = std::to_string(num_mesos);
		string_format::split_number(mesostr);
		mesolabel.change_text(mesostr);
	}

	Button::State UIShop::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		constexpr Range<uint16_t> buy(Buttons::BUY0, Buttons::BUY4);
		constexpr Range<uint16_t> sell(Buttons::SELL0, Buttons::SELL4);
		constexpr Range<uint16_t> recharge(Buttons::RECHARGE0, Buttons::RECHARGE4);

		if (buy.contains(buttonid))
		{
			int16_t selected = buttonid - Buttons::BUY0;
			buystate.select(selected);
			sellstate.selection = -1;

			return Button::State::NORMAL;
		} else if (sell.contains(buttonid))
		{
			int16_t selected = buttonid - Buttons::SELL0;
			sellstate.select(selected);
			buystate.selection = -1;

			return Button::State::NORMAL;
		} else if (recharge.contains(buttonid))
		{
			int16_t row = sellstate.offset + buttonid - Buttons::RECHARGE0;

			if (row < sellstate.lastslot)
				NpcShopActionPacket(sellstate.items[row].get_slot()).dispatch();

			return Button::State::NORMAL;
		} else
		{
			switch (buttonid)
			{
				case Buttons::BUY_ITEM:
					buystate.buy();

					return Button::State::NORMAL;
				case Buttons::SELL_ITEM:
					sellstate.sell(false);

					return Button::State::NORMAL;
				case Buttons::EXIT:
					exit_shop();

					return Button::State::PRESSED;
				case Buttons::EQUIP:
					changeselltab(InventoryType::Id::EQUIP);

					return Button::State::NORMAL;
				case Buttons::USE:
					changeselltab(InventoryType::Id::USE);

					return Button::State::NORMAL;
				case Buttons::SETUP:
					changeselltab(InventoryType::Id::SETUP);

					return Button::State::NORMAL;
				case Buttons::ETC:
					changeselltab(InventoryType::Id::ETC);

					return Button::State::NORMAL;
				case Buttons::CASH:
					changeselltab(InventoryType::Id::CASH);

					return Button::State::NORMAL;
			}
		}

		return Button::State::PRESSED;
	}

	void UIShop::remove_cursor()
	{
		UIDragElement::remove_cursor();

		buyslider.remove_cursor();
		sellslider.remove_cursor();
	}

	Cursor::State UIShop::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursoroffset = cursorpos - position;
		lastcursorpos = cursoroffset;

		if (buyslider.isenabled())
		{
			Cursor::State bstate = buyslider.send_cursor(cursoroffset, clicked);

			if (bstate != Cursor::State::IDLE)
			{
				clear_tooltip();

				return bstate;
			}
		}

		if (sellslider.isenabled())
		{
			Cursor::State sstate = sellslider.send_cursor(cursoroffset, clicked);

			if (sstate != Cursor::State::IDLE)
			{
				clear_tooltip();

				return sstate;
			}
		}

		int16_t xoff = cursoroffset.x();
		int16_t slot = slot_by_position(cursoroffset.y());

		if (slot >= 0)
		{
			if (xoff >= ROW.x() && xoff < ROW.x() + ROW_SIZE.x())
				show_item(slot, true);
			else if (xoff >= ROW.x() + HALF && xoff < ROW.x() + HALF + ROW_SIZE.x())
				show_item(slot, false);
			else
				clear_tooltip();
		} else
		{
			clear_tooltip();
		}

		Cursor::State ret = clicked ? Cursor::State::CLICKING : Cursor::State::IDLE;
		bool handled = false;

		for (size_t i = 0; i < Buttons::NUM_BUTTONS; i++)
		{
			Button &button = *buttons[i];
			bool real = i <= Buttons::RECHARGE4;

			if (button.is_active() && button.bounds(position).contains(cursorpos) && !handled)
			{
				if (button.get_state() == Button::State::NORMAL)
				{
					if (real)
					{
						Sound(Sound::Name::BUTTONOVER).play();
						ret = Cursor::State::CANCLICK;
					}

					button.set_state(Button::State::MOUSEOVER);
				} else if (button.get_state() == Button::State::MOUSEOVER)
				{
					if (clicked)
					{
						if (i >= Buttons::EQUIP && i <= Buttons::CASH)
							Sound(Sound::Name::TAB).play();
						else if (real)
							Sound(Sound::Name::BUTTONCLICK).play();

						button.set_state(button_pressed(i));

						ret = Cursor::State::IDLE;
						// A click on Rech. isn't also one on its row.
						handled = true;
					} else if (real)
					{
						ret = Cursor::State::CANCLICK;
					}
				}
			} else if (button.get_state() == Button::State::MOUSEOVER)
			{
				button.set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	void UIShop::send_scroll(double yoffset)
	{
		int16_t xoff = lastcursorpos.x();

		if (xoff < HALF)
			buyslider.send_scroll(yoffset);
		else
			sellslider.send_scroll(yoffset);
	}

	void UIShop::rightclick(Point<int16_t> cursorpos)
	{
		if (rightclicksell)
		{
			Point<int16_t> cursoroffset = cursorpos - position;

			int16_t xoff = cursoroffset.x();
			int16_t slot = slot_by_position(cursoroffset.y());

			if (slot >= 0 && xoff >= ROW.x() + HALF && xoff < ROW.x() + HALF + ROW_SIZE.x())
			{
				clear_tooltip();

				sellstate.selection = slot + sellstate.offset;
				sellstate.sell(true);
				buystate.selection = -1;
			}
		}
	}

	void UIShop::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
			exit_shop();
	}

	UIElement::Type UIShop::get_type() const
	{
		return TYPE;
	}

	void UIShop::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::SHOP);
	}

	void UIShop::show_item(int16_t slot, bool buy)
	{
		if (buy)
			buystate.show_item(slot);
		else
			sellstate.show_item(slot);
	}

	void UIShop::changeselltab(InventoryType::Id type)
	{
		sellstate.change_tab(inventory, type, meso);

		sellslider.setrows(ROWS, std::max<int16_t>(sellstate.lastslot, ROWS));

		for (int16_t i = 0; i < ROWS; i++)
		{
			if (i < sellstate.lastslot)
				buttons[Buttons::SELL0 + i]->set_state(Button::State::NORMAL);
			else
				buttons[Buttons::SELL0 + i]->set_state(Button::State::DISABLED);
		}

		update_recharge();
	}

	void UIShop::update_recharge()
	{
		for (int16_t i = 0; i < ROWS; i++)
		{
			int16_t row = sellstate.offset + i;
			bool shown = false;

			if (row < sellstate.lastslot)
			{
				const SellItem &item = sellstate.items[row];

				if (const shop_list::Row *shop_row = recharge_row(item.get_id()))
					shown = item.get_sellable() < shop_row->buyable;
			}

			buttons[Buttons::RECHARGE0 + i]->set_active(shown);
		}
	}

	const shop_list::Row *UIShop::recharge_row(int32_t itemid) const
	{
		for (const shop_list::Row &row : recharges)
			if (row.itemid == itemid)
				return &row;

		return nullptr;
	}

	void UIShop::reset(int32_t npcid)
	{
		std::string strid = string_format::extend_id(npcid, 7);
		npc = nl::nx::npc[strid + ".img"]["stand"]["0"];

		for (auto &button : buttons)
			button.second->set_state(Button::State::NORMAL);

		buystate.reset();
		sellstate.reset();
		recharges.clear();

		buyslider.setrows(ROWS, ROWS);

		changeselltab(InventoryType::Id::EQUIP);

		makeactive();
		rightclicksell = Configuration::get().get_rightclicksell();
	}

	void UIShop::modify(InventoryType::Id type)
	{
		if (type == sellstate.tab)
		{
			int16_t offset = sellstate.offset;

			sellstate.change_tab(inventory, type, meso);
			sellstate.offset = std::max<int16_t>(0, std::min<int16_t>(offset, sellstate.lastslot - ROWS));
			sellslider.setrows(sellstate.offset, ROWS, std::max<int16_t>(sellstate.lastslot, ROWS));

			for (int16_t i = 0; i < ROWS; i++)
				buttons[Buttons::SELL0 + i]->set_state(i < sellstate.lastslot ? Button::State::NORMAL
																				: Button::State::DISABLED);

			update_recharge();
		}
	}

	void UIShop::add_item(const shop_list::Row &row)
	{
		if (shop_list::is_rechargeable(row.itemid))
		{
			recharges.push_back(row);
			update_recharge();
		}

		if (!row.for_sale())
			return;

		buystate.add(BuyItem(meso, row));

		buyslider.setrows(ROWS, std::max<int16_t>(buystate.lastslot, ROWS));
	}

	int16_t UIShop::slot_by_position(int16_t y) const
	{
		int16_t yoff = y - ROW.y();

		if (yoff < 0 || yoff % ROW_PITCH >= ROW_SIZE.y())
			return -1;

		int16_t slot = yoff / ROW_PITCH;

		return slot < ROWS ? slot : -1;
	}

	uint16_t UIShop::tabbyinventory(InventoryType::Id type)
	{
		switch (type)
		{
			case InventoryType::Id::USE:
				return Buttons::USE;
			case InventoryType::Id::SETUP:
				return Buttons::SETUP;
			case InventoryType::Id::ETC:
				return Buttons::ETC;
			case InventoryType::Id::CASH:
				return Buttons::CASH;
			default:
				return Buttons::EQUIP;
		}
	}

	void UIShop::exit_shop()
	{
		clear_tooltip();

		deactivate();
		NpcShopActionPacket().dispatch();
	}

	UIShop::BuyItem::BuyItem(Texture cur, const shop_list::Row &r) : currency(cur), row(r)
	{
		namelabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		pricelabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		const ItemData &item = ItemData::get(row.itemid);

		if (item.is_valid())
		{
			icon = item.get_icon(false);
			namelabel.change_text(item.get_name());
		}

		std::string mesostr = std::to_string(row.price);
		string_format::split_number(mesostr);
		pricelabel.change_text(mesostr + "meso");
	}

	void UIShop::BuyItem::draw(Point<int16_t> pos) const
	{
		draw_entry(pos, icon, namelabel, currency, pricelabel);
	}

	int16_t UIShop::BuyItem::get_index() const
	{
		return row.index;
	}

	int32_t UIShop::BuyItem::get_id() const
	{
		return row.itemid;
	}

	int16_t UIShop::BuyItem::get_buyable() const
	{
		return row.buyable;
	}

	UIShop::SellItem::SellItem(int32_t item_id, int16_t count, int16_t s, bool sc, Texture cur) : currency(cur)
	{
		const ItemData &idata = ItemData::get(item_id);

		icon = idata.get_icon(false);
		id = item_id;
		sellable = count;
		slot = s;
		showcount = sc;

		namelabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		pricelabel = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		std::string name = idata.get_name();

		if (name.length() >= 24)
			name = name.substr(0, 22) + "..";

		namelabel.change_text(name);

		int32_t price = idata.get_price();
		std::string mesostr = std::to_string(price);
		string_format::split_number(mesostr);
		pricelabel.change_text(mesostr + "meso");
	}

	void UIShop::SellItem::draw(Point<int16_t> pos) const
	{
		draw_entry(pos, icon, namelabel, currency, pricelabel);

		if (showcount)
		{
			static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"],
													Charset::Alignment::LEFT);
			countset.draw(std::to_string(sellable), pos + ICON + Point<int16_t>(0, 20));
		}
	}

	int32_t UIShop::SellItem::get_id() const
	{
		return id;
	}

	int16_t UIShop::SellItem::get_slot() const
	{
		return slot;
	}

	int16_t UIShop::SellItem::get_sellable() const
	{
		return sellable;
	}

	void UIShop::BuyState::reset()
	{
		items.clear();

		offset = 0;
		lastslot = 0;
		selection = -1;
	}

	void UIShop::BuyState::draw(Point<int16_t> parentpos, const Texture &selected) const
	{
		for (int16_t i = 0; i < ROWS; i++)
		{
			int16_t slot = i + offset;

			if (slot >= lastslot)
				break;

			Point<int16_t> itempos = parentpos + row_position(false, i);

			if (slot == selection)
				selected.draw(itempos + Point<int16_t>(ENTRY_X, 0));

			items[slot].draw(itempos);
		}
	}

	void UIShop::BuyState::show_item(int16_t slot)
	{
		int16_t absslot = slot + offset;

		if (absslot < 0 || absslot >= lastslot)
			return;

		int32_t itemid = items[absslot].get_id();
		UI::get().show_item(Tooltip::Parent::SHOP, itemid);
	}

	void UIShop::BuyState::add(BuyItem item)
	{
		items.push_back(item);

		lastslot++;
	}

	void UIShop::BuyState::buy() const
	{
		if (selection < 0 || selection >= lastslot)
			return;

		const BuyItem &item = items[selection];
		int16_t buyable = item.get_buyable();
		int16_t slot = item.get_index();
		int32_t itemid = item.get_id();

		// Stars and bullets come one full set at a time
		if (buyable > 1 && !shop_list::is_rechargeable(itemid))
		{
			constexpr const char *question = "How many are you willing to buy?";

			auto onenter = [slot, itemid](int32_t qty)
			{
				auto shortqty = static_cast<int16_t>(qty);

				NpcShopActionPacket(slot, itemid, shortqty, true).dispatch();
			};

			UI::get().emplace<UIEnterNumber>(question, onenter, buyable, 1);
		} else if (buyable > 0)
		{
			constexpr const char *question = "Are you sure you want to buy it?";

			auto ondecide = [slot, itemid](bool yes)
			{
				if (yes)
					NpcShopActionPacket(slot, itemid, 1, true).dispatch();
			};

			UI::get().emplace<UIYesNo>(question, ondecide);
		}
	}

	void UIShop::BuyState::select(int16_t selected)
	{
		int16_t slot = selected + offset;

		if (slot == selection)
			buy();
		else
			selection = slot;
	}

	void UIShop::SellState::reset()
	{
		items.clear();

		offset = 0;
		lastslot = 0;
		selection = -1;
		tab = InventoryType::Id::NONE;
	}

	void UIShop::SellState::change_tab(const Inventory &inventory, InventoryType::Id newtab, Texture meso)
	{
		tab = newtab;

		offset = 0;

		items.clear();

		int16_t slots = inventory.get_slotmax(tab);

		for (int16_t i = 1; i <= slots; i++)
		{
			if (int32_t item_id = inventory.get_item_id(tab, i))
			{
				int16_t count = inventory.get_item_count(tab, i);
				items.emplace_back(item_id, count, i, tab != InventoryType::Id::EQUIP, meso);
			}
		}

		lastslot = static_cast<int16_t>(items.size());
	}

	void UIShop::SellState::draw(Point<int16_t> parentpos, const Texture &selected) const
	{
		for (int16_t i = 0; i < ROWS; i++)
		{
			int16_t slot = i + offset;

			if (slot >= lastslot)
				break;

			Point<int16_t> itempos = parentpos + row_position(true, i);

			if (slot == selection)
				selected.draw(itempos + Point<int16_t>(ENTRY_X, 0));

			items[slot].draw(itempos);
		}
	}

	void UIShop::SellState::show_item(int16_t slot)
	{
		int16_t absslot = slot + offset;

		if (absslot < 0 || absslot >= lastslot)
			return;

		if (tab == InventoryType::Id::EQUIP)
		{
			int16_t realslot = items[absslot].get_slot();
			UI::get().show_equip(Tooltip::Parent::SHOP, realslot);
		} else
		{
			int32_t itemid = items[absslot].get_id();
			UI::get().show_item(Tooltip::Parent::SHOP, itemid);
		}
	}

	void UIShop::SellState::sell(bool skip_confirmation) const
	{
		if (selection < 0 || selection >= lastslot)
			return;

		const SellItem &item = items[selection];
		int32_t itemid = item.get_id();
		int16_t sellable = item.get_sellable();
		int16_t slot = item.get_slot();

		// A set of stars or bullets is sold whole
		if (shop_list::is_rechargeable(itemid))
		{
			auto ondecide = [itemid, slot, sellable](bool yes)
			{
				if (yes)
					NpcShopActionPacket(slot, itemid, sellable, false).dispatch();
			};

			if (skip_confirmation)
				ondecide(true);
			else
				UI::get().emplace<UIYesNo>("Are you sure you want to sell it?", ondecide);
		} else if (sellable > 1)
		{
			constexpr const char *question = "How many are you willing to sell?";

			auto onenter = [itemid, slot](int32_t qty)
			{
				auto shortqty = static_cast<int16_t>(qty);

				NpcShopActionPacket(slot, itemid, shortqty, false).dispatch();
			};

			UI::get().emplace<UIEnterNumber>(question, onenter, sellable, 1);
		} else if (sellable > 0)
		{
			if (skip_confirmation)
			{
				NpcShopActionPacket(slot, itemid, 1, false).dispatch();
				return;
			}

			constexpr const char *question = "Are you sure you want to sell it?";

			auto ondecide = [itemid, slot](bool yes)
			{
				if (yes)
					NpcShopActionPacket(slot, itemid, 1, false).dispatch();
			};

			UI::get().emplace<UIYesNo>(question, ondecide);
		}
	}

	void UIShop::SellState::select(int16_t selected)
	{
		int16_t slot = selected + offset;

		if (slot == selection)
			sell(false);
		else
			selection = slot;
	}
}
