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
#include "UIEquipInventory.h"

#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../UITypes/UIItemInventory.h"

#include "../../Audio/Audio.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/InventoryPackets.h"

#include <nlnx/nx.hpp>

#include <array>

namespace ms
{
	namespace
	{
		// The window, measured from period footage. The background labels a
		// grid of cells; the first starts at SLOT and they are SLOT_PITCH apart.
		constexpr Point<int16_t> CLOSE = Point<int16_t>(155, 6);
		constexpr Point<int16_t> SLOT = Point<int16_t>(5, 35);
		constexpr int16_t SLOT_PITCH = 33;
		constexpr int16_t SLOT_SIZE = 32;
		// Where an icon is drawn from, relative to its cell.
		constexpr Point<int16_t> ICON = Point<int16_t>(0, 0);
		constexpr Point<int16_t> PET_SHOW = Point<int16_t>(102, 280);
		// The pet panel hangs off the right edge, flush with the bottom
		// (FUN_007fe02e), with a tab for each pet (FUN_007ffe73).
		constexpr Point<int16_t> PET_PANEL = Point<int16_t>(172, 123);
		constexpr Point<int16_t> PET_TAB = Point<int16_t>(10, 151);
		constexpr int16_t PET_TAB_PITCH = 34;
		constexpr Point<int16_t> PET_HIDE = Point<int16_t>(155, 159);

		struct Cell
		{
			EquipSlot::Id slot;
			int16_t column;
			int16_t row;
		};

		// The cells the background labels. MOB EQUIP and the pet cells have
		// no slot here.
		constexpr Cell CELLS[] = {
				{ EquipSlot::Id::HAT,      1, 0 },
				{ EquipSlot::Id::MEDAL,    0, 1 },
				{ EquipSlot::Id::FACE,     1, 1 },
				{ EquipSlot::Id::RING1,    3, 1 },
				{ EquipSlot::Id::RING2,    4, 1 },
				{ EquipSlot::Id::EYEACC,   2, 2 },
				{ EquipSlot::Id::EARACC,   3, 2 },
				{ EquipSlot::Id::CAPE,     0, 3 },
				{ EquipSlot::Id::TOP,      1, 3 },
				{ EquipSlot::Id::PENDANT1, 2, 3 },
				{ EquipSlot::Id::WEAPON,   3, 3 },
				{ EquipSlot::Id::SHIELD,   4, 3 },
				{ EquipSlot::Id::GLOVES,   0, 4 },
				{ EquipSlot::Id::BOTTOM,   1, 4 },
				{ EquipSlot::Id::BELT,     2, 4 },
				{ EquipSlot::Id::RING3,    3, 4 },
				{ EquipSlot::Id::RING4,    4, 4 },
				{ EquipSlot::Id::SHOES,    2, 5 },
				{ EquipSlot::Id::TAMEDMOB, 0, 6 },
				{ EquipSlot::Id::SADDLE,   1, 6 }
		};

		constexpr Point<int16_t> MOB_EQUIP = Point<int16_t>(2, 6);

		struct PetCell
		{
			// The part it holds on each pet's tab.
			std::array<int16_t, 3> parts;
			Point<int16_t> position;
		};

		// The pet panel's cells (0x00be2260), from its top left.
		constexpr PetCell PET_CELLS[] = {
				{ { 24, 24, 24 }, { 13, 11 } },
				{ { 25, 25, 25 }, { 79, 11 } },
				{ { 22, 33, 41 }, { 13, 44 } },
				{ { 23, 34, 42 }, { 46, 44 } },
				{ { 46, 47, 48 }, { 13, 77 } },
				{ { 28, 37, 45 }, { 46, 77 } },
				{ { 26, 35, 43 }, { 79, 77 } },
				{ { 14, 30, 38 }, { 112, 77 } },
				{ { 27, 36, 44 }, { 13, 110 } },
				{ { 21, 31, 39 }, { 79, 110 } },
				{ { 29, 32, 40 }, { 112, 110 } }
		};

		// The first pet's clothes, label and quote ring cells, and the auto
		// potions' slots beside the HP and MP pouches.
		constexpr Point<int16_t> PET_WEAR = Point<int16_t>(112, 77);
		constexpr Point<int16_t> PET_LABEL = Point<int16_t>(79, 110);
		constexpr Point<int16_t> PET_QUOTE = Point<int16_t>(112, 110);
		constexpr Point<int16_t> HP_POTION = Point<int16_t>(46, 11);
		constexpr Point<int16_t> MP_POTION = Point<int16_t>(112, 11);
		constexpr int16_t HP_POUCH = 24;
		constexpr int16_t MP_POUCH = 25;

		EquipSlot::Id part_slot(int16_t part)
		{
			return static_cast<EquipSlot::Id>(part);
		}

		constexpr Point<int16_t> cell_position(int16_t column, int16_t row)
		{
			return SLOT + Point<int16_t>(column * SLOT_PITCH, row * SLOT_PITCH);
		}

		// Monster Riding for explorers, Cygnus Knights, and Aran.
		constexpr int32_t MONSTER_RIDING[] = { 1004, 10001004, 20001004 };
	}

	UIEquipInventory::UIEquipInventory(const Inventory &invent)
			: UIDragElement<PosEQINV>(), inventory(invent), pet_enabled(false), pet_tab(0)
	{
		for (auto cell : CELLS)
			iconpositions[cell.slot] = cell_position(cell.column, cell.row);

		for (const PetCell &cell : PET_CELLS)
			for (int16_t part : cell.parts)
				iconpositions[part_slot(part)] = PET_PANEL + cell.position;

		nl::node Equip = NxFiles::ui_classic["UIWindow.img"]["Equip"];

		backgrnd = Equip["backgrnd"];
		pet = Equip["pet"];

		// A cell that cannot be used is tinted red, as in period footage.
		unavailable = ColorBox(SLOT_SIZE, SLOT_SIZE, Color::Name::RED, 0.25f);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);
		buttons[Buttons::BT_PET_SHOW] = std::make_unique<MapleButton>(Equip["BtPetEquipShow"], PET_SHOW);

		nl::node hide = Equip["BtPetEquipHide"];
		buttons[Buttons::BT_PET_HIDE] = std::make_unique<MapleButton>(hide,
																	  PET_PANEL + PET_HIDE + Texture(hide["normal"]["0"]).get_origin());

		// Each tab shows a pet's equipment, and is there while the pet is out.
		for (uint16_t i = 0; i < 3; i++)
		{
			nl::node tab = NxFiles::ui_classic["UIWindow.img"]["UserInfo"]["BtPet" + std::to_string(i + 1)];
			Point<int16_t> pos = PET_PANEL + PET_TAB + Point<int16_t>(PET_TAB_PITCH * i, 0);

			buttons[Buttons::BT_PET1 + i] = std::make_unique<MapleButton>(tab, pos + Texture(tab["normal"]["0"]).get_origin());
		}

		Point<int16_t> bg_dimensions = backgrnd.get_dimensions();
		dimension = bg_dimensions;
		dragarea = Point<int16_t>(bg_dimensions.x(), 20);

		load_icons();
		show_pet(false);
	}

	void UIEquipInventory::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		// The panel slides out from under the window (FUN_00801474).
		if (pet_enabled)
		{
			Point<int16_t> panel = position + PET_PANEL;

			pet.draw(panel);

			if (!has_pet(0))
			{
				unavailable.draw(panel + PET_WEAR);
				unavailable.draw(panel + PET_LABEL);
				unavailable.draw(panel + PET_QUOTE);
			}

			// No pouch, no auto potion.
			if (!has_pet(0) || !is_worn(HP_POUCH))
				unavailable.draw(panel + HP_POTION);

			if (!has_pet(0) || !is_worn(MP_POUCH))
				unavailable.draw(panel + MP_POTION);

			for (const PetCell &cell : PET_CELLS)
				if (auto &icon = icons[part_slot(cell.parts[pet_tab])])
					icon->draw(panel + cell.position + ICON);
		}

		backgrnd.draw(position);

		for (auto cell : CELLS)
			if (is_unavailable(cell.slot))
				unavailable.draw(position + iconpositions[cell.slot]);

		if (is_unavailable(EquipSlot::Id::TAMEDMOB))
			unavailable.draw(position + cell_position(MOB_EQUIP.x(), MOB_EQUIP.y()));

		for (auto cell : CELLS)
			if (auto &icon = icons[cell.slot])
				icon->draw(position + iconpositions[cell.slot] + ICON);

		UIElement::draw_buttons(alpha);
	}

	Button::State UIEquipInventory::button_pressed(uint16_t id)
	{
		switch (id)
		{
			case Buttons::BT_CLOSE:
				toggle_active();
				break;
			case Buttons::BT_PET_SHOW:
				show_pet(true);
				break;
			case Buttons::BT_PET_HIDE:
				show_pet(false);
				break;
			case Buttons::BT_PET1:
			case Buttons::BT_PET2:
			case Buttons::BT_PET3:
				pet_tab = id - Buttons::BT_PET1;
				clear_tooltip();
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIEquipInventory::update()
	{
		UIElement::update();

		for (uint8_t i = 0; i < 3; i++)
		{
			Button &tab = *buttons[Buttons::BT_PET1 + i];
			bool out = has_pet(i);

			if (!out)
				tab.set_state(Button::State::DISABLED);
			else if (tab.get_state() == Button::State::DISABLED)
				tab.set_state(Button::State::NORMAL);
		}
	}

	bool UIEquipInventory::has_pet(uint8_t index) const
	{
		return static_cast<bool>(Stage::get().get_player().get_pet(index));
	}

	bool UIEquipInventory::is_worn(int16_t part) const
	{
		return inventory.get_item_id(InventoryType::Id::EQUIPPED, inventory.shown_equip(part_slot(part))) != 0;
	}

	void UIEquipInventory::show_pet(bool show)
	{
		pet_enabled = show;

		buttons[Buttons::BT_PET_SHOW]->set_active(!show);
		buttons[Buttons::BT_PET_HIDE]->set_active(show);

		for (uint16_t i = Buttons::BT_PET1; i <= Buttons::BT_PET3; i++)
			buttons[i]->set_active(show);
	}

	// Pants under an overall, a shield beside a two-handed weapon, and the
	// riding cells without Monster Riding.
	bool UIEquipInventory::is_unavailable(EquipSlot::Id slot) const
	{
		switch (slot)
		{
			case EquipSlot::Id::BOTTOM:
				return inventory.get_item_id(InventoryType::Id::EQUIPPED, EquipSlot::Id::TOP) / 10000 == 105;
			case EquipSlot::Id::SHIELD:
			{
				int32_t weapon = inventory.get_item_id(InventoryType::Id::EQUIPPED, EquipSlot::Id::WEAPON) / 10000;

				return weapon >= 140 && weapon <= 149;
			}
			case EquipSlot::Id::TAMEDMOB:
			case EquipSlot::Id::SADDLE:
			{
				const SkillBook &skills = Stage::get().get_player().get_skills();

				for (int32_t skill : MONSTER_RIDING)
					if (skills.get_level(skill) > 0)
						return false;

				return true;
			}
			default:
				return false;
		}
	}

	void UIEquipInventory::update_slot(EquipSlot::Id slot)
	{
		int16_t shown = inventory.shown_equip(slot);

		if (int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIPPED, shown))
		{
			const Texture &texture = ItemData::get(item_id).get_icon(false);

			icons[slot] = std::make_unique<Icon>(
					std::make_unique<EquipIcon>(shown),
					texture,
					-1
			);
		} else if (icons[slot])
		{
			icons[slot].reset();
		}

		clear_tooltip();
	}

	void UIEquipInventory::load_icons()
	{
		icons.clear();

		for (auto cell : CELLS)
			update_slot(cell.slot);

		for (const PetCell &cell : PET_CELLS)
			for (int16_t part : cell.parts)
				update_slot(part_slot(part));
	}

	Cursor::State UIEquipInventory::send_cursor(bool pressed, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(pressed, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		EquipSlot::Id slot = slot_by_position(cursorpos);

		if (auto icon = icons[slot].get())
		{
			if (pressed)
			{
				icon->start_drag(cursorpos - position - iconpositions[slot] - ICON);

				UI::get().drag_icon(icon);

				clear_tooltip();

				return Cursor::State::GRABBING;
			} else
			{
				show_equip(inventory.shown_equip(slot));

				return Cursor::State::CANGRAB;
			}
		} else
		{
			clear_tooltip();

			return dstate;
		}
	}

	void UIEquipInventory::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			toggle_active();
	}

	UIElement::Type UIEquipInventory::get_type() const
	{
		return TYPE;
	}

	void UIEquipInventory::doubleclick(Point<int16_t> cursorpos)
	{
		EquipSlot::Id slot = slot_by_position(cursorpos);

		if (icons[slot])
			if (int16_t freeslot = inventory.find_free_slot(InventoryType::Id::EQUIP))
				Stage::get().get_player().request(UnequipItemPacket(inventory.shown_equip(slot), freeslot),
												  ExclRequest::WAIT);
	}

	bool UIEquipInventory::is_in_range(Point<int16_t> cursorpos) const
	{
		Rectangle<int16_t> bounds = Rectangle<int16_t>(position, position + dimension);

		if (bounds.contains(cursorpos))
			return true;

		if (!pet_enabled)
			return false;

		Point<int16_t> panel = position + PET_PANEL;

		return Rectangle<int16_t>(panel, panel + pet.get_dimensions()).contains(cursorpos);
	}

	bool UIEquipInventory::send_icon(const Icon& icon, Point<int16_t> cursorpos)
	{
		if (EquipSlot::Id slot = slot_by_position(cursorpos))
			icon.drop_on_equips(slot);

		return true;
	}

	void UIEquipInventory::toggle_active()
	{
		clear_tooltip();

		UIElement::toggle_active();
	}

	void UIEquipInventory::modify(int16_t pos, int8_t mode, int16_t arg)
	{
		EquipSlot::Id eqpos = EquipSlot::by_id(pos % Inventory::CASH_EQUIP);
		EquipSlot::Id eqarg = EquipSlot::by_id(arg % Inventory::CASH_EQUIP);

		switch (mode)
		{
			case 0:
			case 3:
				update_slot(eqpos);
				break;
			case 2:
				update_slot(eqpos);
				update_slot(eqarg);
				break;
		}
	}

	void UIEquipInventory::show_equip(int16_t slot)
	{
		UI::get().show_equip(Tooltip::Parent::EQUIPINVENTORY, slot);
	}

	void UIEquipInventory::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::EQUIPINVENTORY);
	}

	EquipSlot::Id UIEquipInventory::slot_by_position(Point<int16_t> cursorpos) const
	{
		for (auto cell : CELLS)
		{
			Point<int16_t> cellpos = position + iconpositions[cell.slot];
			Rectangle<int16_t> cellrect = Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(SLOT_SIZE, SLOT_SIZE));

			if (cellrect.contains(cursorpos))
				return cell.slot;
		}

		if (!pet_enabled)
			return EquipSlot::Id::NONE;

		for (const PetCell &cell : PET_CELLS)
		{
			Point<int16_t> cellpos = position + PET_PANEL + cell.position;
			Rectangle<int16_t> cellrect = Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(SLOT_SIZE, SLOT_SIZE));

			if (cellrect.contains(cursorpos))
				return part_slot(cell.parts[pet_tab]);
		}

		return EquipSlot::Id::NONE;
	}

	UIEquipInventory::EquipIcon::EquipIcon(int16_t s)
	{
		source = s;
	}

	void UIEquipInventory::EquipIcon::drop_on_stage() const
	{
		Sound(Sound::Name::DRAGEND).play();
	}

	void UIEquipInventory::EquipIcon::drop_on_equips(EquipSlot::Id slot) const
	{
		if (source % Inventory::CASH_EQUIP == slot)
			Sound(Sound::Name::DRAGEND).play();
	}

	bool UIEquipInventory::EquipIcon::drop_on_items(InventoryType::Id tab, EquipSlot::Id eqslot, int16_t slot, bool equip) const
	{
		if (tab != InventoryType::Id::EQUIP)
		{
			if (auto iteminventory = UI::get().get_element<UIItemInventory>())
			{
				if (iteminventory->is_active())
				{
					iteminventory->change_tab(InventoryType::Id::EQUIP);
					return false;
				}
			}
		}

		if (equip)
		{
			// Only an item worn at the same position swaps with this one.
			int16_t worn = Stage::get().get_player().get_inventory().equip_position(slot, eqslot);

			if (worn == source)
				Stage::get().get_player().request(EquipItemPacket(slot, worn), ExclRequest::WAIT);
		} else
		{
			Stage::get().get_player().request(UnequipItemPacket(source, slot), ExclRequest::WAIT);
		}

		return true;
	}

	Icon::IconType UIEquipInventory::EquipIcon::get_type()
	{
		return Icon::IconType::EQUIP;
	}
}