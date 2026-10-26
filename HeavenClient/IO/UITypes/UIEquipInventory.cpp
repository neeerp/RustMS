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
#include "../../Character/MountRules.h"
#include "../../Data/EquipData.h"
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
		// With the second pendant's column (FullBackgrnd), the columns from
		// the fourth on and the close button are a cell to the right, and the
		// window is that much wider (FUN_00801a29).
		constexpr Point<int16_t> CLOSE = Point<int16_t>(155, 6);
		constexpr int16_t HEIGHT = 304;
		constexpr int16_t PENDANT_COLUMN = 3;
		constexpr Point<int16_t> PENDANT2_CELL = Point<int16_t>(PENDANT_COLUMN, 3);
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
		// potions' slots, HP then MP, with the pouches they go beside.
		constexpr Point<int16_t> PET_WEAR = Point<int16_t>(112, 77);
		constexpr Point<int16_t> PET_LABEL = Point<int16_t>(79, 110);
		constexpr Point<int16_t> PET_QUOTE = Point<int16_t>(112, 110);
		constexpr Point<int16_t> POTIONS[] = { Point<int16_t>(46, 11), Point<int16_t>(112, 11) };
		constexpr int16_t POUCHES[] = { 24, 25 };

		EquipSlot::Id part_slot(int16_t part)
		{
			return static_cast<EquipSlot::Id>(part);
		}

		constexpr Point<int16_t> cell_position(int16_t column, int16_t row)
		{
			return SLOT + Point<int16_t>(column * SLOT_PITCH, row * SLOT_PITCH);
		}

		// Where a cell of the narrow window is in this one.
		constexpr Point<int16_t> cell_position(int16_t column, int16_t row, bool wide)
		{
			return cell_position(wide && column >= PENDANT_COLUMN ? column + 1 : column, row);
		}

		// Monster Riding for explorers, Cygnus Knights, and Aran.
		constexpr int32_t MONSTER_RIDING[] = { 1004, 10001004, 20001004 };
	}

	UIEquipInventory::UIEquipInventory(const Inventory &invent)
			: UIDragElement<PosEQINV>(), inventory(invent), pet_enabled(false), wide(invent.has_pendant_cell()), pet_tab(0)
	{
		// v83 hangs the panel at the same place either way (0x7fe142), where
		// the wide window would cover part of it, so here it moves too.
		Point<int16_t> shift = Point<int16_t>(wide ? SLOT_PITCH : 0, 0);
		pet_panel = PET_PANEL + shift;

		for (auto cell : CELLS)
			iconpositions[cell.slot] = cell_position(cell.column, cell.row, wide);

		if (wide)
			iconpositions[EquipSlot::Id::PENDANT2] = cell_position(PENDANT2_CELL.x(), PENDANT2_CELL.y());

		for (const PetCell &cell : PET_CELLS)
			for (int16_t part : cell.parts)
				iconpositions[part_slot(part)] = pet_panel + cell.position;

		nl::node Equip = NxFiles::ui_classic["UIWindow.img"]["Equip"];

		backgrnd = Equip[wide ? "FullBackgrnd" : "backgrnd"];
		pet = Equip["pet"];

		// A cell that cannot be used is tinted red, as in period footage.
		unavailable = ColorBox(SLOT_SIZE, SLOT_SIZE, Color::Name::RED, 0.25f);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE + shift);
		buttons[Buttons::BT_PET_SHOW] = std::make_unique<MapleButton>(Equip["BtPetEquipShow"], PET_SHOW);

		nl::node hide = Equip["BtPetEquipHide"];
		buttons[Buttons::BT_PET_HIDE] = std::make_unique<MapleButton>(hide,
																	  pet_panel + PET_HIDE + Texture(hide["normal"]["0"]).get_origin());

		// Each tab shows a pet's equipment, and is there while the pet is out.
		for (uint16_t i = 0; i < 3; i++)
		{
			nl::node tab = NxFiles::ui_classic["UIWindow.img"]["UserInfo"]["BtPet" + std::to_string(i + 1)];
			Point<int16_t> pos = pet_panel + PET_TAB + Point<int16_t>(PET_TAB_PITCH * i, 0);

			buttons[Buttons::BT_PET1 + i] = std::make_unique<MapleButton>(tab, pos + Texture(tab["normal"]["0"]).get_origin());
		}

		// The window keeps its height though the wide art is shorter.
		Point<int16_t> bg_dimensions = backgrnd.get_dimensions();
		dimension = Point<int16_t>(bg_dimensions.x(), HEIGHT);
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
			Point<int16_t> panel = position + pet_panel;

			pet.draw(panel);

			if (!has_pet(0))
			{
				unavailable.draw(panel + PET_WEAR);
				unavailable.draw(panel + PET_LABEL);
				unavailable.draw(panel + PET_QUOTE);
			}

			// The potions and how many are left, even none (0x80190c). No
			// pouch, no auto potion.
			for (uint8_t mp = 0; mp < 2; mp++)
			{
				if (potions[mp])
					potions[mp]->draw(panel + POTIONS[mp] + ICON);

				if (!has_pet(0) || !is_worn(POUCHES[mp]))
					unavailable.draw(panel + POTIONS[mp]);
			}

			for (const PetCell &cell : PET_CELLS)
				if (auto &icon = icons[part_slot(cell.parts[pet_tab])])
					icon->draw(panel + cell.position + ICON);
		}

		backgrnd.draw(position);

		for (EquipSlot::Id slot : cells())
			if (is_unavailable(slot))
				unavailable.draw(position + iconpositions[slot]);

		if (is_unavailable(EquipSlot::Id::TAMEDMOB))
			unavailable.draw(position + cell_position(MOB_EQUIP.x(), MOB_EQUIP.y()));

		for (EquipSlot::Id slot : cells())
			if (auto &icon = icons[slot])
				icon->draw(position + iconpositions[slot] + ICON);

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

		update_potions();
	}

	void UIEquipInventory::update_potions()
	{
		Player &player = Stage::get().get_player();

		for (uint8_t mp = 0; mp < 2; mp++)
		{
			// v83 clears a potion each time the panel is drawn without the
			// first pet or its pouch (FUN_00801474); this tells the server once.
			// The pets are only away while the field changes.
			if (pet_enabled && Stage::get().is_active() && player.get_auto_potion(mp) && !player.wears_pouch(mp))
				player.change_auto_potion(mp, 0);

			int32_t item_id = player.get_auto_potion(mp);
			int16_t count = item_id ? inventory.get_total_item_count(item_id) : 0;

			if (item_id != potion_ids[mp])
			{
				potion_ids[mp] = item_id;

				// A potion held on the cursor goes with its icon.
				if (potions[mp] && potions[mp]->get_drag())
					UI::get().cancel_drag();

				if (item_id)
					potions[mp] = std::make_unique<Icon>(std::make_unique<PotionIcon>(mp),
														 ItemData::get(item_id).get_icon(false), count);
				else
					potions[mp].reset();
			}
			else if (potions[mp])
			{
				potions[mp]->set_count(count);
			}
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

	// Pants under an overall, a shield beside a two-handed weapon, the
	// riding cells without Monster Riding, and the second pendant's while
	// shut (0x7ff006).
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
			case EquipSlot::Id::PENDANT2:
				return !inventory.extra_pendant_open();
			default:
				return false;
		}
	}

	std::vector<EquipSlot::Id> UIEquipInventory::cells() const
	{
		std::vector<EquipSlot::Id> slots;

		for (auto cell : CELLS)
			slots.push_back(cell.slot);

		if (wide)
			slots.push_back(EquipSlot::Id::PENDANT2);

		return slots;
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

		for (EquipSlot::Id slot : cells())
			update_slot(slot);

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

		int8_t potion = potion_by_position(cursorpos);

		if (potion >= 0 && potions[potion])
		{
			clear_tooltip();

			if (!pressed)
				return Cursor::State::CANGRAB;

			potions[potion]->start_drag(cursorpos - position - pet_panel - POTIONS[potion] - ICON);

			UI::get().drag_icon(potions[potion].get());

			return Cursor::State::GRABBING;
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

		if (!icons[slot])
			return;

		int16_t shown = inventory.shown_equip(slot);
		int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIPPED, shown);

		if (slot == EquipSlot::Id::TAMEDMOB && mount::stays_on(item_id))
			return;

		// v83 counts the free slots first, then gets off a tamed monster
		// instead of taking its mount or saddle off (0x4f0e97).
		int16_t freeslot = inventory.find_free_slot(InventoryType::Id::EQUIP);

		if (!freeslot || Stage::get().get_player().ride_keeps(item_id))
			return;

		Stage::get().get_player().request(UnequipItemPacket(shown, freeslot), ExclRequest::WAIT);
	}

	bool UIEquipInventory::is_in_range(Point<int16_t> cursorpos) const
	{
		Rectangle<int16_t> bounds = Rectangle<int16_t>(position, position + dimension);

		if (bounds.contains(cursorpos))
			return true;

		if (!pet_enabled)
			return false;

		Point<int16_t> panel = position + pet_panel;

		return Rectangle<int16_t>(panel, panel + pet.get_dimensions()).contains(cursorpos);
	}

	bool UIEquipInventory::send_icon(const Icon& icon, Point<int16_t> cursorpos)
	{
		// Nothing is dropped on the second pendant's cell; a pendant gets
		// there only when the first is taken (report b178).
		if (int8_t potion = potion_by_position(cursorpos); potion >= 0)
			icon.drop_on_potion(potion);
		else if (EquipSlot::Id slot = slot_by_position(cursorpos); slot && slot != EquipSlot::Id::PENDANT2)
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
		for (EquipSlot::Id slot : cells())
		{
			Point<int16_t> cellpos = position + iconpositions[slot];
			Rectangle<int16_t> cellrect = Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(SLOT_SIZE, SLOT_SIZE));

			if (cellrect.contains(cursorpos))
				return slot;
		}

		if (!pet_enabled)
			return EquipSlot::Id::NONE;

		for (const PetCell &cell : PET_CELLS)
		{
			Point<int16_t> cellpos = position + pet_panel + cell.position;
			Rectangle<int16_t> cellrect = Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(SLOT_SIZE, SLOT_SIZE));

			if (cellrect.contains(cursorpos))
				return part_slot(cell.parts[pet_tab]);
		}

		return EquipSlot::Id::NONE;
	}

	int8_t UIEquipInventory::potion_by_position(Point<int16_t> cursorpos) const
	{
		if (!pet_enabled)
			return -1;

		for (int8_t mp = 0; mp < 2; mp++)
		{
			Point<int16_t> cellpos = position + pet_panel + POTIONS[mp];

			if (Rectangle<int16_t>(cellpos, cellpos + Point<int16_t>(SLOT_SIZE, SLOT_SIZE)).contains(cursorpos))
				return mp;
		}

		return -1;
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

	bool UIEquipInventory::EquipIcon::drop_on_items(InventoryType::Id tab, EquipSlot::Id, int16_t slot, bool equip) const
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

		Player &player = Stage::get().get_player();

		// Off a tamed monster first, before the swap is weighed (0x4f11b4).
		if (player.ride_keeps(player.get_inventory().get_item_id(InventoryType::Id::EQUIPPED, source)))
			return true;

		if (equip && !can_swap(slot))
			return true;

		// Onto equipment, the server swaps the two (FUN_004f11b4).
		Stage::get().get_player().request(UnequipItemPacket(source, slot), ExclRequest::WAIT);

		return true;
	}

	bool UIEquipInventory::EquipIcon::can_swap(int16_t slot) const
	{
		// v83 refuses a swap without a word.
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIP, slot);

		// It has to go where this is worn; any ring goes on any finger.
		auto finger = [](int16_t part)
		{
			return part == EquipSlot::Id::RING2 || part == EquipSlot::Id::RING3 || part == EquipSlot::Id::RING4
				? EquipSlot::Id::RING1 : part;
		};

		EquipSlot::Id part = EquipData::get(item_id).get_eqslot();

		if (finger(part) != finger(source % Inventory::CASH_EQUIP)
			|| inventory.equip_position(slot, part) / Inventory::CASH_EQUIP != source / Inventory::CASH_EQUIP)
			return false;

		// An overall takes off pants, and a two-handed weapon a shield, which
		// need a free slot (FUN_004f2f55, FUN_004f2fdc).
		bool displaces = source < Inventory::CASH_EQUIP
			&& ((item_id / 10000 == 105 && inventory.has_equipped(EquipSlot::Id::BOTTOM))
				|| (item_id / 100000 == 14 && inventory.has_equipped(EquipSlot::Id::SHIELD)));

		if (displaces && !inventory.find_free_slot(InventoryType::Id::EQUIP))
			return false;

		return UIItemInventory::meets_requirements(item_id);
	}

	Icon::IconType UIEquipInventory::EquipIcon::get_type()
	{
		return Icon::IconType::EQUIP;
	}

	UIEquipInventory::PotionIcon::PotionIcon(bool m) : mp(m) {}

	void UIEquipInventory::PotionIcon::drop_on_stage() const
	{
		Stage::get().get_player().change_auto_potion(mp, 0);
	}

	Icon::IconType UIEquipInventory::PotionIcon::get_type()
	{
		return Icon::IconType::EQUIP;
	}
}