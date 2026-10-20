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
#include "InventoryHandlers.h"

#include "Helpers/ItemParser.h"

#include "../Gameplay/Stage.h"
#include "../IO/UI.h"
#include "../IO/Messages.h"

#include "../Character/Inventory/Inventory.h"
#include "../IO/UITypes/UIEquipInventory.h"
#include "../IO/UITypes/UIItemInventory.h"
#include "../IO/UITypes/UIKeyConfig.h"
#include "../IO/UITypes/UIShop.h"
#include "../IO/UITypes/UIStatusMessenger.h"
#include "../IO/UITypes/UIStorage.h"
#include "../IO/UITypes/UITeleportRock.h"
#include "../IO/UITypes/UINotice.h"
#include "../Character/Pets.h"

namespace ms
{
	void GatherResultHandler::handle(InPacket&) const
	{
		if (auto iteminventory = UI::get().get_element<UIItemInventory>())
			iteminventory->set_sort(true);
	}

	void SortResultHandler::handle(InPacket&) const
	{
		if (auto iteminventory = UI::get().get_element<UIItemInventory>())
		{
			iteminventory->set_sort(false);
			iteminventory->clear_new();
		}
	}

	void InventoryGrowHandler::handle(InPacket& recv) const
	{
		InventoryType::Id type = InventoryType::by_value(recv.read_byte());
		uint8_t slots = recv.read_byte();

		Stage::get().get_player().get_inventory().set_slotmax(type, slots);

		if (auto iteminventory = UI::get().get_element<UIItemInventory>())
			iteminventory->refresh_slots();
	}

	void MapTransferResultHandler::handle(InPacket& recv) const
	{
		int8_t result = recv.read_byte();
		bool vip = recv.read_bool();
		TeleportRock &rock = Stage::get().get_player().get_teleport_rock();

		if (result == TeleportRock::DROPPED || result == TeleportRock::KEPT)
		{
			std::vector<int32_t> maps;

			for (size_t i = 0; i < TeleportRock::slots(vip); i++)
				maps.push_back(recv.read_int());

			rock.set_maps(vip, maps);

			if (auto window = UI::get().get_element<UITeleportRock>(); window && window->is_active())
				window->refresh();
		} else if (std::string notice = TeleportRock::refusal(result, rock.get_target()); !notice.empty())
		{
			UI::get().emplace<UIOk>(notice, [](bool)
			{});
		}

		rock.set_target("");
	}

	void ModifyInventoryHandler::handle(InPacket& recv) const
	{
		// Set when this answers the player's item request.
		if (recv.read_bool())
			Stage::get().get_player().request_answered();

		// A dragged icon points into the inventory windows' icons, which the
		// changes below rebuild; drop the drag while it is still valid.
		UI::get().cancel_drag();

		Inventory& inventory = Stage::get()
			.get_player()
			.get_inventory();

		// A pet's item sent again may carry new closeness (FUN_00704c18).
		std::array<int32_t, 3> closeness = Stage::get().get_player().get_pet_closeness();

		struct Mod
		{
			int8_t mode;
			InventoryType::Id type;
			int16_t pos;
			int16_t arg;
		};

		std::vector<Mod> mods;

		int8_t size = recv.read_byte();

		for (int8_t i = 0; i < size; i++)
		{
			Mod mod;
			mod.mode = recv.read_byte();
			mod.type = InventoryType::by_value(recv.read_byte());
			mod.pos = recv.read_short();

			mod.arg = 0;

			switch (mod.mode)
			{
			case Inventory::Modification::ADD:
				ItemParser::parse_item(recv, mod.type, mod.pos, inventory);

				if (auto keyconfig = UI::get().get_element<UIKeyConfig>())
				{
					int16_t count_now = inventory.get_item_count(mod.type, mod.pos);
					keyconfig->update_item_count(mod.type, mod.pos, count_now);
				}

				break;
			case Inventory::Modification::CHANGECOUNT:
			{
				mod.arg = recv.read_short();

				int16_t count_before = inventory.get_item_count(mod.type, mod.pos);
				int16_t count_now = mod.arg;

				inventory.modify(mod.type, mod.pos, mod.mode, mod.arg, Inventory::Movement::MOVE_NONE);

				if (auto keyconfig = UI::get().get_element<UIKeyConfig>())
					keyconfig->update_item_count(mod.type, mod.pos, count_now - count_before);

				if (count_before < count_now)
					mod.mode = Inventory::Modification::ADDCOUNT;
			}
			break;
			case Inventory::Modification::SWAP:
				mod.arg = recv.read_short();
				break;
			case Inventory::Modification::REMOVE:
			{
				// Like v83, losing the last stack of the chair the player sits
				// on asks the server to let them up.
				Player& player = Stage::get().get_player();
				int32_t itemid = inventory.get_item_id(mod.type, mod.pos);

				if (itemid != 0 && itemid == player.get_chair() && inventory.count_slots(mod.type, itemid) == 1)
					player.leave_chair();

				if (auto keyconfig = UI::get().get_element<UIKeyConfig>())
				{
					int16_t count_before = inventory.get_item_count(mod.type, mod.pos);
					keyconfig->update_item_count(mod.type, mod.pos, -1 * count_before);
				}

				inventory.modify(mod.type, mod.pos, mod.mode, mod.arg, Inventory::Movement::MOVE_INTERNAL);
			}
			break;
			}

			mods.push_back(mod);
		}

		Inventory::Movement move = (recv.length() > 0) ?
			Inventory::movementbyvalue(recv.read_byte()) :
			Inventory::Movement::MOVE_INTERNAL;

		for (const Mod& mod : mods)
		{
			// Like v83 (FUN_00a1ead9), the trailing byte only concerns the
			// entries at a worn (negative) position; the others, such as the
			// scroll used up next to a scrolled equip, are bag changes. A move
			// between the bag and a worn position goes the way its signs say,
			// as a two-handed weapon put on takes the shield off in the same
			// packet.
			Inventory::Movement mod_move = Inventory::Movement::MOVE_INTERNAL;

			if (mod.mode == Inventory::Modification::SWAP && (mod.pos < 0) != (mod.arg < 0))
				mod_move = mod.pos < 0 ? Inventory::Movement::MOVE_UNEQUIP : Inventory::Movement::MOVE_EQUIP;
			else if (mod.pos < 0 || mod.arg < 0)
				mod_move = move;

			if (mod.mode == 2)
				inventory.modify(mod.type, mod.pos, mod.mode, mod.arg, mod_move);

			if (auto shop = UI::get().get_element<UIShop>())
				shop->modify(mod.type);

			if (auto storage = UI::get().get_element<UIStorage>())
				storage->modify(mod.type);

			auto eqinvent = UI::get().get_element<UIEquipInventory>();
			auto itinvent = UI::get().get_element<UIItemInventory>();

			switch (mod_move)
			{
			case Inventory::Movement::MOVE_INTERNAL:
				switch (mod.type)
				{
				case InventoryType::Id::EQUIPPED:
					if (eqinvent)
						eqinvent->modify(mod.pos, mod.mode, mod.arg);

					Stage::get().get_player().change_equip(-mod.pos);
					Stage::get().get_player().change_equip(-mod.arg);
					break;
				case InventoryType::Id::EQUIP:
				case InventoryType::Id::USE:
				case InventoryType::Id::SETUP:
				case InventoryType::Id::ETC:
				case InventoryType::Id::CASH:
					if (itinvent)
						itinvent->modify(mod.type, mod.pos, mod.mode, mod.arg);

					break;
				}

				break;
			case Inventory::Movement::MOVE_EQUIP:
			case Inventory::Movement::MOVE_UNEQUIP:
				if (mod.pos < 0)
				{
					if (eqinvent)
						eqinvent->modify(-mod.pos, 3, 0);

					if (itinvent)
						itinvent->modify(InventoryType::Id::EQUIP, mod.arg, mod.mode, 0);

					Stage::get().get_player().change_equip(-mod.pos);
				}
				else if (mod.arg < 0)
				{
					if (eqinvent)
						eqinvent->modify(-mod.arg, 0, 0);

					if (itinvent)
						itinvent->modify(InventoryType::Id::EQUIP, mod.pos, Inventory::Modification::REMOVE, 0);

					Stage::get().get_player().change_equip(-mod.arg);
				}

				break;
			}
		}

		Stage::get().get_player().recalc_stats();

		std::array<int32_t, 3> now = Stage::get().get_player().get_pet_closeness();

		for (size_t i = 0; i < now.size(); i++)
		{
			if (closeness[i] < 0 || now[i] < 0)
				continue;

			if (std::string line = pets::closeness_change(closeness[i], now[i]); !line.empty())
				if (auto messenger = UI::get().get_element<UIStatusMessenger>())
					messenger->show_status(Color::Name::YELLOW, line);
		}

		// As after a stat change, v83 checks the alert's quests again (FUN_00a1ead9 -> FUN_00a0843c).
		Stage::get().get_quest_alert().stats_changed();

		UI::get().enable();
	}
}