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
#include "UIItemInventory.h"

#include "UIChatBar.h"
#include "UINotice.h"
#include "UIPersonalShop.h"

#include "../Messages.h"
#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"
#include "../UITypes/UIKeyConfig.h"

#include "../../Character/Pets.h"
#include "../../Character/Inventory/ItemDrop.h"
#include "../../Character/Inventory/ScrollRules.h"
#include "../../Data/EquipData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Handlers/MiniRoom.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Net/Packets/MiniRoomPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, measured from period footage. The full window keeps the
		// small one's left side; its title buttons follow the right edge.
		constexpr int16_t CLOSE_RIGHT = 19;
		constexpr int16_t TITLE_BUTTON_PITCH = 15;
		constexpr int16_t TITLE_BUTTON_Y = 12;
		constexpr Point<int16_t> TAB_STRIP = Point<int16_t>(3, 23);
		constexpr int16_t TAB_FILL = 22;
		// Each tab's highlight for a new item; the selected tab's is a pixel taller.
		constexpr Point<int16_t> NEW_TAB = Point<int16_t>(0, 1);
		// The top-left of the first slot, and the distance between slots.
		constexpr Point<int16_t> SLOT = Point<int16_t>(8, 50);
		constexpr Point<int16_t> FULL_SLOT = Point<int16_t>(7, 50);
		constexpr Point<int16_t> SLOT_PITCH = Point<int16_t>(36, 34);
		constexpr int16_t PAGE_PITCH = 149;
		constexpr int16_t SLIDER_X = 152;
		constexpr Range<int16_t> SLIDER_RANGE = Range<int16_t>(51, 237);
		constexpr Point<int16_t> COIN = Point<int16_t>(8, 267);
		constexpr Point<int16_t> FULL_COIN = Point<int16_t>(7, 267);
		// The right end of the meso amount, which sits in a field ending at 137.
		constexpr Point<int16_t> MESO = Point<int16_t>(131, 260);
		constexpr Point<int16_t> FULL_MESO = Point<int16_t>(135, 260);

		// v83's notices 0x10a and 0x42 (FUN_004f5697).
		constexpr const char *CATCH_BREATH = "You can't do it right now.\\nCatch your breath and then try again.";
		constexpr const char *DEAD = "You can't do it once you're dead.";
		// StringPool 0x180 and 0x181.
		constexpr const char *TOO_MANY_PETS = "You have exceeded the maximum number\\n of pets that can accompany you.\\nPlease try again after \\nyou change the living pet back into a doll.";
		// StringPool 0xed8.
		constexpr const char *CASH_FOOD_NO_PET = "This item can only be used with a pet.";
		constexpr const char *BOSS_PET = "Would you like to change your Boss Pet?\\n (The \"Item Ignore\" skill, as well as \\nthe stats for speed and jump will \\nfollow that of the Lead Pet)";

		// v83 FUN_00a240a2: double-clicking a pet asks the server to bring it
		// out, or back if it is out.
		void summon_pet(int16_t slot, int64_t serial)
		{
			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::USE_WAIT))
				return;

			if (player.get_look().get_alerted())
			{
				UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
				{});
				return;
			}

			int32_t lead = pets::follow_the_lead(player.get_stats().get_job().get_id());

			switch (pets::summon(player.get_skilllevel(lead) > 0, player.get_pet_serials(), serial))
			{
				case pets::Summon::SEND:
					player.request(SpawnPetPacket(slot, false), ExclRequest::USE_WAIT);
					break;
				case pets::Summon::FULL:
					UI::get().emplace<UIOk>(TOO_MANY_PETS, [](bool)
					{});
					break;
				case pets::Summon::ASK_BOSS:
					// Any answer but Yes brings it out behind the others.
					UI::get().emplace<UIYesNo>(BOSS_PET, [slot](bool yes)
					{
						Stage::get().get_player().request(SpawnPetPacket(slot, yes), ExclRequest::USE_WAIT);
					});
					break;
			}
		}

		// v83 FUN_00a0a63f, type 0x1e: cash pet food needs a pet out, and the
		// server picks the pet it suits. Using it plays the shop sound; the
		// pet's eat sound waits for CASH_PET_FOOD_RESULT.
		void feed_cash_food(int16_t slot, int32_t item_id)
		{
			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::WAIT))
				return;

			if (!player.get_pet_serials()[0])
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(CASH_FOOD_NO_PET, UIChatBar::LineType::SKILL);

				return;
			}

			if (player.request(UseCashItemPacket(slot, item_id), ExclRequest::WAIT))
				Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
		}

		// v83 FUN_00a0900a: a map may keep items off the ground, which it
		// says only once the player has answered every question.
		void send_drop(InventoryType::Id tab, int16_t slot, int16_t qty)
		{
			if (Stage::get().get_mapinfo().can_drop())
				Stage::get().get_player().request(MoveItemPacket(tab, slot, 0, qty), ExclRequest::WAIT);
			else
				InChatMessage(Messages::DROP_FIELD).drop();
		}
	}

	UIItemInventory::UIItemInventory(const Inventory &invent)
			: UIDragElement<PosINV>(), inventory(invent), tab(InventoryType::Id::EQUIP), ignore_tooltip(false),
			  sort_enabled(false), full_enabled(false)
	{
		nl::node Item = NxFiles::ui_classic["UIWindow.img"]["Item"];

		backgrnd = Item["backgrnd"];
		full_backgrnd = Item["FullBackgrnd"];

		tabs = ClassicTabs(ClassicTabs::TAB5, Item["Tab"]["enabled"], Item["Tab"]["disabled"], TAB_FILL);

		nl::node New = Item["New"];
		newitemslot = New["inventory"];
		newitemtab[false] = New["Tab0"];
		newitemtab[true] = New["Tab1"];

		projectile = Item["activeIcon"];
		disabled = Item["disabled"];

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"]);

		for (uint16_t i = 0; i < TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), tabs.height()));
			buttons[Buttons::BT_TAB0 + i]->set_tab();
		}

		buttons[Buttons::BT_COIN] = std::make_unique<MapleButton>(Item["BtCoin"]);
		buttons[Buttons::BT_GATHER] = std::make_unique<MapleButton>(Item["BtGather"]);
		buttons[Buttons::BT_SORT] = std::make_unique<MapleButton>(Item["BtSort"]);
		buttons[Buttons::BT_FULL] = std::make_unique<MapleButton>(Item["BtFull"]);
		buttons[Buttons::BT_SMALL] = std::make_unique<MapleButton>(Item["BtSmall"]);

		mesolabel = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		for (uint16_t i = 0; i < TABS; i++)
			rows[static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + i)] = 0;

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, ROWS, 1,
				[&](bool upwards)
				{
					int16_t row = rows[tab] + (upwards ? -1 : 1);
					int16_t last = (inventory.get_slotmax(tab) + COLUMNS - 1) / COLUMNS - ROWS;

					if (row >= 0 && row <= last)
						rows[tab] = row;
				}
		);

		set_full(false);
		update_slider();
		clear_new();
	}

	void UIItemInventory::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		if (full_enabled)
			full_backgrnd.draw(position);
		else
			backgrnd.draw(position);

		uint16_t selected = tab - InventoryType::Id::EQUIP;
		tabs.draw(position + TAB_STRIP, TABS, selected);

		if (newtab != InventoryType::Id::NONE)
		{
			uint16_t index = newtab - InventoryType::Id::EQUIP;
			bool is_selected = index == selected;
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * index, 0) + NEW_TAB;

			if (is_selected)
				tab_pos.shift_y(-1);

			newitemtab[is_selected].draw(position + tab_pos, alpha);
			tabs.draw_label(position + TAB_STRIP, index, is_selected);
		}

		if (full_enabled)
		{
			mesolabel.draw(position + FULL_MESO);
		} else
		{
			slider.draw(position);
			mesolabel.draw(position + MESO);
		}

		int16_t numslots = inventory.get_slotmax(tab);

		for (int16_t slot = first_slot(); slot <= last_slot(); slot++)
		{
			Point<int16_t> slotpos = position + get_slotpos(slot);

			if (slot > numslots)
			{
				disabled.draw(slotpos);
				continue;
			}

			auto iter = icons.find(slot);

			if (iter != icons.end() && iter->second)
				iter->second->draw(slotpos);
		}

		int16_t bulletslot = inventory.get_bulletslot();

		if (tab == InventoryType::Id::USE && is_visible(bulletslot))
			projectile.draw(position + get_slotpos(bulletslot));

		if (tab == newtab && is_visible(newslot))
			newitemslot.draw(position + get_slotpos(newslot), alpha);

		UIElement::draw_buttons(alpha);
	}

	void UIItemInventory::update()
	{
		UIElement::update();

		newitemtab[false].update(6);
		newitemtab[true].update(6);
		newitemslot.update(6);

		std::string meso_str = std::to_string(inventory.get_meso());
		string_format::split_number(meso_str);

		mesolabel.change_text(meso_str);
	}

	void UIItemInventory::update_slot(int16_t slot)
	{
		if (int32_t item_id = inventory.get_item_id(tab, slot))
		{
			int16_t count;

			if (tab == InventoryType::Id::EQUIP)
				count = -1;
			else
				count = inventory.get_item_count(tab, slot);

			const ItemData &idata = ItemData::get(item_id);
			const bool questitem = idata.is_questitem();
			const bool untradable = idata.is_untradable();
			const bool cashitem = idata.is_cashitem() || inventory.get_cash_id(tab, slot) != 0;
			const Texture &texture = idata.get_icon(false);
			EquipSlot::Id eqslot = inventory.find_equipslot(item_id);

			icons[slot] = std::make_unique<Icon>(
					std::make_unique<ItemIcon>(*this, tab, eqslot, slot, item_id, count, questitem, untradable,
											   cashitem),
					texture, count
			);
		} else if (icons.count(slot))
		{
			icons.erase(slot);
		}
	}

	void UIItemInventory::load_icons()
	{
		icons.clear();

		int16_t numslots = inventory.get_slotmax(tab);

		for (int16_t slot = 1; slot <= numslots; slot++)
			update_slot(slot);
	}

	void UIItemInventory::refresh_slots()
	{
		load_icons();
		update_slider();
	}

	void UIItemInventory::update_slider()
	{
		int16_t total = (inventory.get_slotmax(tab) + COLUMNS - 1) / COLUMNS;
		slider.setrows(rows[tab], ROWS, std::max<int16_t>(total, ROWS));
	}

	Button::State UIItemInventory::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_CLOSE:
				toggle_active();
				break;
			case Buttons::BT_TAB0:
			case Buttons::BT_TAB1:
			case Buttons::BT_TAB2:
			case Buttons::BT_TAB3:
			case Buttons::BT_TAB4:
			{
				auto type = static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + buttonid - Buttons::BT_TAB0);

				if (type != tab)
				{
					clear_tooltip();

					tab = type;

					update_slider();
					load_icons();
					set_sort(false);
				}

				// The strip is drawn from tab, not the button states. A tab
				// left PRESSED would ignore every later click (#188).
				break;
			}
			case Buttons::BT_GATHER:
				Stage::get().get_player().request(GatherItemsPacket(tab), ExclRequest::WAIT);
				break;
			case Buttons::BT_SORT:
				Stage::get().get_player().request(SortItemsPacket(tab), ExclRequest::WAIT);
				break;
			case Buttons::BT_FULL:
				set_full(true);
				break;
			case Buttons::BT_SMALL:
				set_full(false);
				break;
			case Buttons::BT_COIN:
			{
				// One drop holds 10 to 50,000 mesos, and the box starts at 10.
				constexpr int32_t MESO_DROP_MIN = 10;
				constexpr int32_t MESO_DROP_MAX = 50000;

				auto most = static_cast<int32_t>(std::min<int64_t>(inventory.get_meso(), MESO_DROP_MAX));

				if (most >= MESO_DROP_MIN)
				{
					auto onenter = [](int32_t amount)
					{
						if (Stage::get().get_mapinfo().can_drop())
							Stage::get().get_player().request(MesoDropPacket(amount), ExclRequest::USE_WAIT);
						else
							InChatMessage(Messages::DROP_FIELD).drop();
					};

					UI::get().emplace<UIEnterNumber>(
							"How many will you drop? (A portion of the amount may be lost if picked up by another user.)",
							onenter, most, MESO_DROP_MIN, MESO_DROP_MIN);
				}

				break;
			}
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIItemInventory::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t slot = slot_by_position(cursorpos - position);

		if (icons.count(slot) && is_visible(slot))
		{
			if (int32_t item_id = inventory.get_item_id(tab, slot))
			{
				// v83 FUN_004efd25 refuses everything once the player is dead.
				if (Stage::get().get_player().is_dead())
				{
					UI::get().emplace<UIOk>(DEAD, [](bool)
					{});
					return;
				}

				switch (tab)
				{
					case InventoryType::Id::EQUIP:
						if (can_wear_equip(slot))
							Stage::get().get_player().request(EquipItemPacket(slot, inventory.find_equipslot(item_id)), ExclRequest::WAIT);

						break;
					case InventoryType::Id::USE:
						if (!Stage::get().get_mapinfo().can_use_item(item_id))
							InChatMessage(Messages::ITEM_FIELD).drop();
						else if (!disease::can_use_item(Stage::get().get_player().get_diseases()))
							break;
						else if (pets::is_food(item_id))
							Stage::get().get_player().feed_pets(slot, item_id);
						else
							Stage::get().get_player().request(UseItemPacket(slot, item_id), ExclRequest::USE_WAIT);

						break;
					case InventoryType::Id::SETUP:
						Stage::get().get_player().use_item(item_id);
						break;
					case InventoryType::Id::CASH:
						// As in v83, a face item (516xxxx) makes its emote
						// (id % 100 + 8) as a face key would, without being
						// used up.
						if (item_id / 10000 == 516)
							Stage::get().send_key(KeyType::Id::FACE, 99 + item_id % 100 + 8, true);
						else if (personal_shop::is_permit(item_id))
							UIPersonalShop::create(slot, item_id);
						else if (item_id / 10000 == 500)
							summon_pet(slot, inventory.get_cash_id(tab, slot));
						else if (pets::is_cash_food(item_id))
							feed_cash_food(slot, item_id);

						break;
				}
			}
		}
	}

	bool UIItemInventory::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		// A row of my shop goes back anywhere on the window, once it shows
		// the row's tab (FUN_004f4b49).
		if (InventoryType::Id back = icon.get_return_tab(); back != InventoryType::Id::NONE)
		{
			if (back != tab)
			{
				change_tab(back);

				return false;
			}

			icon.drop_on_inventory();

			return true;
		}

		int16_t slot = slot_by_position(cursorpos - position);

		if (slot > 0)
		{
			int32_t item_id = inventory.get_item_id(tab, slot);
			EquipSlot::Id eqslot;
			bool equip;

			if (item_id && tab == InventoryType::Id::EQUIP)
			{
				eqslot = inventory.find_equipslot(item_id);
				equip = true;
			} else
			{
				eqslot = EquipSlot::Id::NONE;
				equip = false;
			}

			ignore_tooltip = true;

			return icon.drop_on_items(tab, eqslot, slot, equip);
		}

		return true;
	}

	Cursor::State UIItemInventory::send_cursor(bool pressed, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(pressed, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		Point<int16_t> cursor_relative = cursorpos - position;

		if (!full_enabled && slider.isenabled())
		{
			Cursor::State sstate = slider.send_cursor(cursor_relative, pressed);

			if (sstate != Cursor::State::IDLE)
			{
				clear_tooltip();

				return sstate;
			}
		}

		int16_t slot = slot_by_position(cursor_relative);
		Icon *icon = get_icon(slot);
		bool is_icon = icon && is_visible(slot);

		if (is_icon)
		{
			if (pressed)
			{
				Point<int16_t> slotpos = get_slotpos(slot);
				icon->start_drag(cursor_relative - slotpos);
				UI::get().drag_icon(icon);

				clear_tooltip();

				return Cursor::State::GRABBING;
			} else if (!ignore_tooltip)
			{
				show_item(slot);

				return Cursor::State::CANGRAB;
			} else
			{
				ignore_tooltip = false;

				return Cursor::State::CANGRAB;
			}
		} else
		{
			clear_tooltip();

			return UIElement::send_cursor(pressed, cursorpos);
		}
	}

	void UIItemInventory::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
			{
				toggle_active();
			} else if (keycode == KeyAction::Id::TAB)
			{
				uint16_t next = (tab - InventoryType::Id::EQUIP + 1) % TABS;

				button_pressed(Buttons::BT_TAB0 + next);
			}
		}
	}

	UIElement::Type UIItemInventory::get_type() const
	{
		return TYPE;
	}

	void UIItemInventory::modify(InventoryType::Id type, int16_t slot, int8_t mode, int16_t arg)
	{
		if (slot <= 0)
			return;

		if (type == tab)
		{
			switch (mode)
			{
				case Inventory::Modification::ADD:
					update_slot(slot);
					break;
				case Inventory::Modification::CHANGECOUNT:
				case Inventory::Modification::ADDCOUNT:
					if (auto icon = get_icon(slot))
						icon->set_count(arg);

					break;
				case Inventory::Modification::SWAP:
					if (arg != slot)
					{
						update_slot(slot);
						update_slot(arg);
					}

					break;
				case Inventory::Modification::REMOVE:
					update_slot(slot);
					break;
			}
		}

		switch (mode)
		{
			case Inventory::Modification::ADD:
			case Inventory::Modification::ADDCOUNT:
				newtab = type;
				newslot = slot;
				break;
			case Inventory::Modification::CHANGECOUNT:
			case Inventory::Modification::SWAP:
			case Inventory::Modification::REMOVE:
				if (newslot == slot && newtab == type)
					clear_new();

				break;
		}
	}

	void UIItemInventory::set_sort(bool enabled)
	{
		sort_enabled = enabled;

		buttons[Buttons::BT_GATHER]->set_active(!sort_enabled);
		buttons[Buttons::BT_SORT]->set_active(sort_enabled);
	}

	void UIItemInventory::change_tab(InventoryType::Id type)
	{
		if (type >= InventoryType::Id::EQUIP && type <= InventoryType::Id::CASH)
			button_pressed(Buttons::BT_TAB0 + type - InventoryType::Id::EQUIP);
	}

	void UIItemInventory::clear_new()
	{
		newtab = InventoryType::Id::NONE;
		newslot = 0;
	}

	void UIItemInventory::toggle_active()
	{
		UIElement::toggle_active();

		if (!active)
		{
			clear_new();
			clear_tooltip();
		}
	}

	void UIItemInventory::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
	}

	void UIItemInventory::show_item(int16_t slot)
	{
		if (tab == InventoryType::Id::EQUIP)
		{
			UI::get().show_equip(Tooltip::Parent::ITEMINVENTORY, slot);
		} else
		{
			int32_t item_id = inventory.get_item_id(tab, slot);
			UI::get().show_item(Tooltip::Parent::ITEMINVENTORY, item_id);
		}
	}

	void UIItemInventory::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::ITEMINVENTORY);
	}

	bool UIItemInventory::is_visible(int16_t slot) const
	{
		return slot >= first_slot() && slot <= last_slot();
	}

	int16_t UIItemInventory::first_slot() const
	{
		return full_enabled ? 1 : 1 + rows.at(tab) * COLUMNS;
	}

	int16_t UIItemInventory::last_slot() const
	{
		return first_slot() + (full_enabled ? MAXFULLSLOTS : MAXSLOTS) - 1;
	}

	bool UIItemInventory::can_wear_equip(int16_t slot) const
	{
		const Player &player = Stage::get().get_player();
		const CharStats &stats = player.get_stats();
		const CharLook &look = player.get_look();

		if (look.get_alerted())
		{
			UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
			{});
			return false;
		}

		if (player.is_dead())
		{
			UI::get().emplace<UIOk>(DEAD, [](bool)
			{});
			return false;
		}

		const int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIP, slot);
		const EquipData &equipdata = EquipData::get(item_id);
		const ItemData &itemdata = equipdata.get_itemdata();

		const int8_t reqGender = itemdata.get_gender();
		const bool female = stats.get_female();

		switch (reqGender)
		{
			case 0: // Male
				if (female)
					return false;

				break;
			case 1: // Female
				if (!female)
					return false;

				break;
			case 2: // Unisex
			default:
				break;
		}

		const std::string jobname = stats.get_jobname();

		if (jobname == "GM" || jobname == "SuperGM")
			return true;

		int16_t reqJOB = equipdata.get_reqstat(MapleStat::Id::JOB);

		if (!stats.get_job().is_sub_job(reqJOB))
		{
			UI::get().emplace<UIOk>("Your current job\\ncannot equip the selected item.", [](bool)
			{});
			return false;
		}

		int16_t reqLevel = equipdata.get_reqstat(MapleStat::Id::LEVEL);
		int16_t reqDEX = equipdata.get_reqstat(MapleStat::Id::DEX);
		int16_t reqSTR = equipdata.get_reqstat(MapleStat::Id::STR);
		int16_t reqLUK = equipdata.get_reqstat(MapleStat::Id::LUK);
		int16_t reqINT = equipdata.get_reqstat(MapleStat::Id::INT);
		int16_t reqFAME = equipdata.get_reqstat(MapleStat::Id::FAME);

		int8_t i = 0;

		if (reqLevel > stats.get_stat(MapleStat::Id::LEVEL))
			i++;
		else if (reqDEX > stats.get_total(EquipStat::Id::DEX))
			i++;
		else if (reqSTR > stats.get_total(EquipStat::Id::STR))
			i++;
		else if (reqLUK > stats.get_total(EquipStat::Id::LUK))
			i++;
		else if (reqINT > stats.get_total(EquipStat::Id::INT))
			i++;
		else if (reqFAME > stats.get_honor())
			i++;

		if (i > 0)
		{
			UI::get().emplace<UIOk>(
					"Your stats are too low to equip this item\\nor you do not meet the job requirement.", [](bool)
					{});
			return false;
		}

		return true;
	}

	int16_t UIItemInventory::slot_by_position(Point<int16_t> cursorpos) const
	{
		Point<int16_t> offset = cursorpos - (full_enabled ? FULL_SLOT : SLOT);

		if (offset.x() < 0 || offset.y() < 0)
			return 0;

		int16_t page = 0;

		if (full_enabled)
		{
			page = offset.x() / PAGE_PITCH;
			offset.shift_x(-page * PAGE_PITCH);
		}

		int16_t column = offset.x() / SLOT_PITCH.x();
		int16_t row = offset.y() / SLOT_PITCH.y();

		if (page >= (full_enabled ? PAGES : 1) || column >= COLUMNS || row >= ROWS)
			return 0;

		int16_t slot = first_slot() + page * MAXSLOTS + row * COLUMNS + column;

		return is_visible(slot) ? slot : 0;
	}

	Point<int16_t> UIItemInventory::get_slotpos(int16_t slot) const
	{
		int16_t index = slot - first_slot();
		int16_t page = index / MAXSLOTS;
		int16_t column = index % COLUMNS;
		int16_t row = index % MAXSLOTS / COLUMNS;

		Point<int16_t> first = full_enabled ? FULL_SLOT : SLOT;

		return first + Point<int16_t>(page * PAGE_PITCH + column * SLOT_PITCH.x(), row * SLOT_PITCH.y());
	}

	Icon *UIItemInventory::get_icon(int16_t slot)
	{
		auto iter = icons.find(slot);

		if (iter != icons.end())
			return iter->second.get();
		else
			return nullptr;
	}

	void UIItemInventory::set_full(bool enabled)
	{
		full_enabled = enabled;

		dimension = (full_enabled ? full_backgrnd : backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		int16_t close_x = dimension.x() - CLOSE_RIGHT;
		Point<int16_t> title_button = Point<int16_t>(close_x - TITLE_BUTTON_PITCH + 6, TITLE_BUTTON_Y);

		buttons[Buttons::BT_CLOSE]->set_position(Point<int16_t>(close_x, 6));
		buttons[Buttons::BT_GATHER]->set_position(title_button);
		buttons[Buttons::BT_SORT]->set_position(title_button);
		buttons[Buttons::BT_FULL]->set_position(title_button - Point<int16_t>(TITLE_BUTTON_PITCH, 0));
		buttons[Buttons::BT_SMALL]->set_position(title_button - Point<int16_t>(TITLE_BUTTON_PITCH, 0));
		buttons[Buttons::BT_COIN]->set_position(full_enabled ? FULL_COIN : COIN);

		buttons[Buttons::BT_FULL]->set_active(!full_enabled);
		buttons[Buttons::BT_SMALL]->set_active(full_enabled);

		set_sort(sort_enabled);
		load_icons();
	}

	void UIItemInventory::ItemIcon::drop_on_trade(int8_t slot) const
	{
		constexpr const char *trademessage = "How many will you trade?";

		Player &player = Stage::get().get_player();

		if (!player.can_request(ExclRequest::WAIT))
			return;

		auto tab = static_cast<int8_t>(sourcetab);

		// v83 puts up one of anything that doesn't ask; the server takes
		// equipment and whole stacks of stars and bullets.
		if (mini_room::asks_count(item_id, count))
		{
			auto onenter = [tab, source = source, slot](int32_t qty)
			{
				Stage::get().get_player().request(TradePutItemPacket(tab, source, qty, slot), ExclRequest::WAIT);
			};

			UI::get().emplace<UIEnterNumber>(trademessage, onenter, count, count);
		} else
		{
			player.request(TradePutItemPacket(tab, source, 1, slot), ExclRequest::WAIT);
		}
	}

	void UIItemInventory::ItemIcon::drop_on_shop() const
	{
		// v83 refuses these before asking anything (FUN_004f41b9).
		if (questitem || untradable || cashitem)
		{
			UI::get().emplace<UIOk>("This item is unavailable to trade with others.", [](bool) {});

			return;
		}

		if (auto shop = UI::get().get_element<UIPersonalShop>())
			shop->sell(sourcetab, source, item_id, count);
	}

	void UIItemInventory::ItemIcon::set_count(int16_t c)
	{
		count = c;
	}

	Icon::IconType UIItemInventory::ItemIcon::get_type()
	{
		return Icon::IconType::ITEM;
	}

	UIItemInventory::ItemIcon::ItemIcon(const UIItemInventory &parent, InventoryType::Id st, EquipSlot::Id eqs,
										int16_t s, int32_t iid, int16_t c, bool q, bool u, bool cash) : parent(parent)
	{
		sourcetab = st;
		eqsource = eqs;
		source = s;
		item_id = iid;
		count = c;
		questitem = q;
		untradable = u;
		cashitem = cash;
	}

	void UIItemInventory::ItemIcon::drop_on_stage() const
	{
		constexpr const char *dropmessage = "How many will you drop?";
		constexpr const char *untradablemessage = "This item can't be taken back once\\nthrown away. Will you still drop it?";
		constexpr const char *cashmessage = "You can't drop this item.";

		const Player &player = Stage::get().get_player();

		if (player.get_look().get_alerted())
		{
			UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
			{});

			return;
		}

		if (player.is_dead())
		{
			UI::get().emplace<UIOk>(DEAD, [](bool)
			{});

			return;
		}

		if (item_drop::refused(item_id, cashitem))
		{
			UI::get().emplace<UIOk>(cashmessage, [](bool)
			{});

			return;
		}

		auto drop = [tab = sourcetab, slot = source, id = item_id, max = count, dropmessage]()
		{
			if (item_drop::asks_count(id, max))
			{
				auto onenter = [tab, slot](int32_t qty)
				{
					send_drop(tab, slot, qty);
				};

				UI::get().emplace<UIEnterNumber>(dropmessage, onenter, max, max);
			} else
			{
				send_drop(tab, slot, 1);
			}
		};

		// As in v83, a quest item that is also untradable asks twice.
		if (uint8_t asks = item_drop::asks(questitem, untradable))
		{
			auto onok = [drop](bool ok)
			{
				if (ok)
					drop();
			};

			UI::get().emplace<UIYesNo>(untradablemessage, onok, Text::Alignment::CENTER, asks);
		} else
		{
			drop();
		}
	}

	void UIItemInventory::ItemIcon::drop_on_equips(EquipSlot::Id eqslot) const
	{
		switch (sourcetab)
		{
			case InventoryType::Id::EQUIP:
				if (eqsource == eqslot)
					if (parent.can_wear_equip(source))
						Stage::get().get_player().request(EquipItemPacket(source, eqslot), ExclRequest::WAIT);

				Sound(Sound::Name::DRAGEND).play();

				break;
			case InventoryType::Id::USE:
			{
				// v83 FUN_004f4bb7: a scroll dropped on a worn equip. HeavenClient
				// has no sealed items, so v83's notices about those don't apply.
				Player &player = Stage::get().get_player();

				if (player.get_look().get_alerted())
				{
					UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
					{});
					break;
				}

				if (player.is_dead())
				{
					UI::get().emplace<UIOk>(DEAD, [](bool)
					{});
					break;
				}

				if (!scroll_rules::is_scroll(item_id) || !player.can_request(ExclRequest::WAIT))
					break;

				const Inventory &inventory = player.get_inventory();

				// Also refuses an empty slot.
				if (!scroll_rules::fits(item_id, inventory.get_item_id(InventoryType::Id::EQUIPPED, eqslot)))
					break;

				auto send = [slot = source, eqslot](bool whitescroll)
				{
					Stage::get().get_player().request(ScrollEquipPacket(slot, eqslot, whitescroll), ExclRequest::WAIT);
				};

				// Either answer scrolls; only Yes uses the White Scroll.
				if (scroll_rules::takes_white_scroll(item_id) &&
					inventory.find_item(InventoryType::Id::USE, scroll_rules::WHITE_SCROLL) != 0)
					UI::get().emplace<UIYesNo>("Would you like to use the White Scroll?", send);
				else
					send(false);

				break;
			}
		}
	}

	bool UIItemInventory::ItemIcon::drop_on_items(InventoryType::Id tab, EquipSlot::Id, int16_t slot, bool) const
	{
		if (tab != sourcetab || slot == source)
			return true;

		Stage::get().get_player().request(MoveItemPacket(tab, source, slot, 1), ExclRequest::WAIT);

		return true;
	}

	void UIItemInventory::ItemIcon::drop_on_bindings(Point<int16_t> cursorposition, bool remove) const
	{
		Keyboard::Mapping mapping = get_mapping();

		if (mapping.type == KeyType::Id::NONE)
			return;

		auto keyconfig = UI::get().get_element<UIKeyConfig>();

		if (remove)
			keyconfig->unstage_mapping(mapping);
		else
			keyconfig->stage_mapping(cursorposition, mapping);
	}

	// Only use and set-up items go on keys.
	Keyboard::Mapping UIItemInventory::ItemIcon::get_mapping() const
	{
		if (sourcetab == InventoryType::Id::USE || sourcetab == InventoryType::Id::SETUP)
			return Keyboard::Mapping(KeyType::ITEM, item_id);

		return {};
	}
}
