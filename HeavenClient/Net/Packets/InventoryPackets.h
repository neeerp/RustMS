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

#include "../OutPacket.h"

#include "../../Character/Inventory/Inventory.h"

namespace ms
{
	// Packet which requests that the inventory is sorted
	// Opcode: GATHER_ITEMS(69)
	class GatherItemsPacket : public OutPacket
	{
	public:
		GatherItemsPacket(InventoryType::Id type) : OutPacket(OutPacket::Opcode::GATHER_ITEMS)
		{
			write_time();
			write_byte(type);
		}
	};

	// Packet which requests that the inventory is sorted
	// Opcode: SORT_ITEMS(70)
	class SortItemsPacket : public OutPacket
	{
	public:
		SortItemsPacket(InventoryType::Id type) : OutPacket(OutPacket::Opcode::SORT_ITEMS)
		{
			write_time();
			write_byte(type);
		}
	};

	// Packet which requests that an item is moved
	// Opcode: MOVE_ITEM(71)
	class MoveItemPacket : public OutPacket
	{
	public:
		MoveItemPacket(InventoryType::Id type, int16_t slot, int16_t action, int16_t qty) : OutPacket(OutPacket::Opcode::MOVE_ITEM)
		{
			write_time();
			write_byte(type);
			write_short(slot);
			write_short(action);
			write_short(qty);
		}
	};

	// Packet which requests that mesos are dropped on the map
	// Opcode: MESO_DROP(94)
	class MesoDropPacket : public OutPacket
	{
	public:
		MesoDropPacket(int32_t amount) : OutPacket(OutPacket::Opcode::MESO_DROP)
		{
			write_time();
			write_int(amount);
		}
	};

	// Packet which requests that an item is equipped at worn position `dest`
	// (Inventory::equip_position).
	// Opcode: MOVE_ITEM(71)
	class EquipItemPacket : public MoveItemPacket
	{
	public:
		EquipItemPacket(int16_t src, int16_t dest) : MoveItemPacket(InventoryType::Id::EQUIP, src, -dest, 1) {}
	};

	// Packet which requests that an item is unequipped
	// Opcode: MOVE_ITEM(71)
	class UnequipItemPacket : public MoveItemPacket
	{
	public:
		UnequipItemPacket(int16_t src, int16_t dest) : MoveItemPacket(InventoryType::Id::EQUIPPED, -src, dest, 1) {}
	};

	// A packet which requests that an 'USE' item is used
	// Opcode: USE_ITEM(72)
	class UseItemPacket : public OutPacket
	{
	public:
		UseItemPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_ITEM)
		{
			Sound(itemid).play();

			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Feeds the pet food (212xxxx) in USE slot `slot` to the pet out that
	// is least full (FUN_00a09905)
	// Opcode: PET_FOOD(76)
	class PetFoodPacket : public OutPacket
	{
	public:
		PetFoodPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::PET_FOOD)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Uses the item in Cash slot `slot` (FUN_00a0a63f). The item's own fields
	// go between its id and the tick; cash pet food (524xxxx) has none.
	// Opcode: USE_CASH_ITEM(79)
	class UseCashItemPacket : public OutPacket
	{
	public:
		UseCashItemPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_time();
		}

		// The Pet Name Tag (5170000) sends the pet's new name.
		UseCashItemPacket(int16_t slot, int32_t itemid, const std::string &text)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_string(text);
			write_time();
		}
	};

	// Uses a super megaphone (FUN_00a0a63f type 0xd): the line, and
	// whether it shows an ear to whisper back by.
	// Opcode: USE_CASH_ITEM(79)
	class UseSuperMegaphonePacket : public OutPacket
	{
	public:
		UseSuperMegaphonePacket(int16_t slot, int32_t itemid, const std::string &text, bool ear)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_string(text);
			write_byte(ear);
			write_time();
		}
	};

	// Uses an item megaphone (FUN_005ec4b7): a super megaphone's line that
	// may show the item in `item_slot` of `tab`, or none for NONE.
	// Opcode: USE_CASH_ITEM(79)
	class UseItemMegaphonePacket : public OutPacket
	{
	public:
		UseItemMegaphonePacket(int16_t slot, int32_t itemid, const std::string &text, bool ear, InventoryType::Id tab,
							   int16_t item_slot) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_string(text);
			write_byte(ear);
			write_byte(tab != InventoryType::Id::NONE);

			if (tab != InventoryType::Id::NONE)
			{
				write_int(tab);
				write_int(item_slot);
			}

			write_time();
		}
	};

	// Uses a triple megaphone (FUN_00a0a63f type 0x3c): one to three lines.
	// Opcode: USE_CASH_ITEM(79)
	class UseTripleMegaphonePacket : public OutPacket
	{
	public:
		UseTripleMegaphonePacket(int16_t slot, int32_t itemid, const std::vector<std::string> &lines, bool ear)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_byte(static_cast<int8_t>(lines.size()));

			for (const std::string &line : lines)
				write_string(line);

			write_byte(ear);
			write_time();
		}
	};

	// Uses the teleport rock (504xxxx) in Cash slot `slot` to go to a
	// character, or to a kept map (FUN_00a0a4aa).
	// Opcode: USE_CASH_ITEM(79)
	class UseTeleportRockPacket : public OutPacket
	{
	public:
		UseTeleportRockPacket(int16_t slot, int32_t itemid, const std::string &name)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_byte(1);
			write_string(name);
			write_time();
		}

		UseTeleportRockPacket(int16_t slot, int32_t itemid, int32_t mapid) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_byte(0);
			write_int(mapid);
			write_time();
		}
	};

	// Keeps the player's map for a teleport rock, or drops a kept one
	// (FUN_00a261bc). The server answers with the list.
	// Opcode: TELEPORT_ROCK_LIST(102)
	class TeleportRockListPacket : public OutPacket
	{
	public:
		// Keeps the map the player is on.
		TeleportRockListPacket(bool vip) : OutPacket(OutPacket::Opcode::TELEPORT_ROCK_LIST)
		{
			write_byte(1);
			write_byte(vip);
		}

		// Drops `mapid`.
		TeleportRockListPacket(bool vip, int32_t mapid) : OutPacket(OutPacket::Opcode::TELEPORT_ROCK_LIST)
		{
			write_byte(0);
			write_byte(vip);
			write_int(mapid);
		}
	};

	// Requests reading a skill or mastery book (v83 FUN_00a0a1b2)
	// Opcode: USE_SKILL_BOOK(82)
	class SkillBookPacket : public OutPacket
	{
	public:
		SkillBookPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_SKILL_BOOK)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Requests using the scroll in a USE slot on a worn equip
	// Opcode: SCROLL_EQUIP(86), v83 FUN_00a09221
	class ScrollEquipPacket : public OutPacket
	{
	public:
		ScrollEquipPacket(int16_t source, int16_t target, bool whitescroll) : OutPacket(OutPacket::Opcode::SCROLL_EQUIP)
		{
			write_time();
			write_short(source);
			write_short(-target);
			write_short(whitescroll ? 2 : 1);
			write_byte(0); // Legendary Spirit
		}
	};
}