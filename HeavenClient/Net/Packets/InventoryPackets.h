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

#include <array>
#include <cstdint>

#include "../Login.h"
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

		// The Item Tag (5060000) and Magical Sandglasses (550xxxx) send the worn equip's slot, negative as
		// in moves.
		UseCashItemPacket(int16_t slot, int32_t itemid, int16_t equipped) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_short(-equipped);
			write_time();
		}
	};

	// Sends a note with the note item (509xxxx, v83 a0b6e4).
	// Opcode: USE_CASH_ITEM(79)
	class SendNotePacket : public OutPacket
	{
	public:
		SendNotePacket(int16_t slot, int32_t itemid, const std::string &to, const std::string &text)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_string(to);
			write_string(text);
			write_time();
		}
	};

	// Uses a jukebox (FUN_00a0a63f type 0x14): the song's length in ms
	// (inferred).
	// Opcode: USE_CASH_ITEM(79)
	class UseJukeboxPacket : public OutPacket
	{
	public:
		UseJukeboxPacket(int16_t slot, int32_t itemid, int32_t length) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_int(length);
			write_time();
		}
	};

	// Uses an AP or SP Reset (FUN_00a0a63f types 0x17 and 0x18): where the
	// AP or skill level goes, then where it leaves, as STAT_CHANGED mask
	// bits or skill ids.
	// Opcode: USE_CASH_ITEM(79)
	class UseResetPacket : public OutPacket
	{
	public:
		UseResetPacket(int16_t slot, int32_t itemid, int32_t to, int32_t from)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_int(to);
			write_int(from);
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

	// Uses an avatar megaphone (FUN_00a0a63f's 539 branch): four lines, the
	// ear, and my look in its megaphone form (FUN_004e73b4), the reverse of
	// LoginParser::parse_look.
	// Opcode: USE_CASH_ITEM(79)
	class UseAvatarMegaphonePacket : public OutPacket
	{
	public:
		UseAvatarMegaphonePacket(int16_t slot, int32_t itemid, const std::array<std::string, 4> &lines, bool ear,
								 const LookEntry &look) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);

			for (const std::string &line : lines)
				write_string(line);

			write_byte(ear);
			write_byte(look.female);
			write_byte(look.skin);
			write_int(look.faceid);
			write_byte(0);
			write_int(look.hairid);

			for (auto [part, id] : look.equips)
			{
				write_byte(part);
				write_int(id);
			}

			write_byte(-1);

			int32_t cash_weapon = 0;

			for (auto [part, id] : look.maskedequips)
			{
				if (part == -111)
				{
					cash_weapon = id;
				} else
				{
					write_byte(part);
					write_int(id);
				}
			}

			write_byte(-1);
			write_int(cash_weapon);

			for (size_t i = 0; i < 3; i++)
				write_int(i < look.petids.size() ? look.petids[i] : 0);

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

	// Uses the teleport rock (504xxxx) in Cash slot `slot`, or the USE-tab
	// rock (232xxxx, FUN_00a0a3bb) in USE slot `slot`, to go to a character,
	// or to a kept map (FUN_00a0a4aa).
	// Opcode: USE_CASH_ITEM(79), USE_TELEPORT_ROCK(84)
	class UseTeleportRockPacket : public OutPacket
	{
	public:
		UseTeleportRockPacket(int16_t slot, int32_t itemid, const std::string &name) : OutPacket(opcode(itemid))
		{
			write_short(slot);
			write_int(itemid);
			write_byte(1);
			write_string(name);
			write_time();
		}

		UseTeleportRockPacket(int16_t slot, int32_t itemid, int32_t mapid) : OutPacket(opcode(itemid))
		{
			write_short(slot);
			write_int(itemid);
			write_byte(0);
			write_int(mapid);
			write_time();
		}

	private:
		static OutPacket::Opcode opcode(int32_t itemid)
		{
			return itemid / 10000 == 232 ? OutPacket::Opcode::USE_TELEPORT_ROCK : OutPacket::Opcode::USE_CASH_ITEM;
		}
	};

	// The Owl of Minerva's search window opened and asks for the items
	// searched for most (FUN_008a0e9a).
	// Opcode: OWL_OPEN(66)
	class OwlOpenPacket : public OutPacket
	{
	public:
		OwlOpenPacket() : OutPacket(OutPacket::Opcode::OWL_OPEN)
		{
			write_byte(5);
		}
	};

	// Searches the shops for `searchid` with the Owl in USE slot `slot`,
	// cheapest first unless `descending` (FUN_00a0a25e, FUN_008a2407).
	// Opcode: USE_OWL(83)
	class UseOwlPacket : public OutPacket
	{
	public:
		UseOwlPacket(int16_t slot, int32_t itemid, int32_t searchid, bool descending) : OutPacket(OutPacket::Opcode::USE_OWL)
		{
			write_short(slot);
			write_int(itemid);
			write_int(searchid);
			write_byte(descending);
			write_time();
		}
	};

	// GO on a search row: the shop's room and map (FUN_008a4423).
	// Opcode: OWL_GO(67)
	class OwlGoPacket : public OutPacket
	{
	public:
		OwlGoPacket(int32_t room, int32_t mapid) : OutPacket(OutPacket::Opcode::OWL_GO)
		{
			write_int(room);
			write_int(mapid);
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

	// Requests reading a return scroll (203xxxx), v83 FUN_00a1e1ce
	// Opcode: USE_RETURN_SCROLL(85)
	class ReturnScrollPacket : public OutPacket
	{
	public:
		ReturnScrollPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_RETURN_SCROLL)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Requests feeding the tamed monster ridden the food (226xxxx) in a USE
	// slot, v83 FUN_00a09a64
	// Opcode: USE_MOUNT_FOOD(77)
	class MountFoodPacket : public OutPacket
	{
	public:
		MountFoodPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_MOUNT_FOOD)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Requests throwing the catch item (227xxxx) in a USE slot at the mob
	// `oid`, found within the item's area, v83 FUN_00a09bdf
	// Opcode: USE_CATCH_ITEM(81)
	class UseCatchItemPacket : public OutPacket
	{
	public:
		UseCatchItemPacket(int16_t slot, int32_t itemid, int32_t oid) : OutPacket(OutPacket::Opcode::USE_CATCH_ITEM)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
			write_int(oid);
		}
	};

	// Requests opening the summoning sack (210xxxx) in a USE slot, v83
	// FUN_00a0977b
	// Opcode: USE_SUMMON_BAG(75)
	class SummonSackPacket : public OutPacket
	{
	public:
		SummonSackPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_SUMMON_BAG)
		{
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// Requests using the Writ of Solomon (237xxxx) in a USE slot, whose EXP
	// becomes gacha EXP, v83 FUN_00a12685
	// Opcode: USE_SOLOMON_ITEM(157)
	class SolomonItemPacket : public OutPacket
	{
	public:
		SolomonItemPacket(int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::USE_SOLOMON_ITEM)
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

	// Uses the scroll in USE `scroll` on the equip in Equip-tab `equip`, from Legendary Spirit's window
	// (FUN_00a09221 with legendary 1).
	// Opcode: SCROLL_EQUIP(86)
	class LegendaryScrollPacket : public OutPacket
	{
	public:
		LegendaryScrollPacket(int16_t scroll, int16_t equip, bool whitescroll)
				: OutPacket(OutPacket::Opcode::SCROLL_EQUIP)
		{
			write_time();
			write_short(scroll);
			write_short(equip);
			write_short(whitescroll ? 2 : 1);
			write_byte(1);
		}
	};

	// Uses the cash item in Cash `slot` on the item in `target_slot` of `tab`:
	// an Item Guard (v83 CUIItemProtector's Yes, 0x82a2ee) or the Scissors
	// of Karma (0x830fb5).
	// Opcode: USE_CASH_ITEM(79)
	class UseOnItemPacket : public OutPacket
	{
	public:
		UseOnItemPacket(int16_t slot, int32_t itemid, InventoryType::Id tab, int16_t target_slot)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_int(tab);
			write_int(target_slot);
			write_time();
		}
	};

	// Uses Vicious' Hammer on the equip in `equip_slot` of the Equip tab
	// (v83 FUN_0082aed3, BtStart).
	// Opcode: USE_CASH_ITEM(79)
	class UseHammerPacket : public OutPacket
	{
	public:
		UseHammerPacket(int16_t slot, int32_t itemid, int16_t equip_slot) : OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_int(InventoryType::Id::EQUIP);
			write_int(equip_slot);
			write_time();
		}
	};

	// Tells the server the hammer's window has shown what came of it, with
	// the code and flag the server sent (FUN_0082ae28).
	// Opcode: USE_HAMMER(260)
	class HammerShownPacket : public OutPacket
	{
	public:
		HammerShownPacket(int32_t code, int32_t flag) : OutPacket(OutPacket::Opcode::USE_HAMMER)
		{
			write_int(code);
			write_int(flag);
		}
	};

	// Asks the Item Maker to make `itemid` from its recipe, with a stimulator
	// or not and the gems put in (FUN_00827096, mode 1).
	// Opcode: ITEM_MAKE(113)
	class ItemMakePacket : public OutPacket
	{
	public:
		ItemMakePacket(int32_t itemid, bool stimulator, const std::vector<int32_t>& gems) : OutPacket(OutPacket::Opcode::ITEM_MAKE)
		{
			write_int(1);
			write_int(itemid);
			write_byte(stimulator);
			write_int(static_cast<int32_t>(gems.size()));

			for (int32_t gem : gems)
				write_int(gem);
		}
	};

	// Asks the Item Maker for a monster crystal from 100 of the drop
	// `itemid` (FUN_00827096, mode 3).
	// Opcode: ITEM_MAKE(113)
	class ItemMakeCrystalPacket : public OutPacket
	{
	public:
		ItemMakeCrystalPacket(int32_t itemid) : OutPacket(OutPacket::Opcode::ITEM_MAKE)
		{
			write_int(3);
			write_int(itemid);
		}
	};

	// Asks the Item Maker to take apart the equipment `itemid` in `slot` of
	// the Equip tab (FUN_00827096, mode 4).
	// Opcode: ITEM_MAKE(113)
	class ItemDisassemblePacket : public OutPacket
	{
	public:
		ItemDisassemblePacket(int32_t itemid, int16_t slot) : OutPacket(OutPacket::Opcode::ITEM_MAKE)
		{
			write_int(4);
			write_int(itemid);
			write_int(1);
			write_int(slot);
		}
	};

	// Casts Vega's Spell: the scroll in `scroll_slot` of the Use tab on the
	// equip in `equip_slot` of the Equip tab (FUN_0082cbe2).
	// Opcode: USE_CASH_ITEM(79)
	class UseVegaSpellPacket : public OutPacket
	{
	public:
		UseVegaSpellPacket(int16_t slot, int32_t itemid, int16_t equip_slot, int16_t scroll_slot)
				: OutPacket(OutPacket::Opcode::USE_CASH_ITEM)
		{
			write_short(slot);
			write_int(itemid);
			write_int(InventoryType::Id::EQUIP);
			write_int(equip_slot);
			write_int(InventoryType::Id::USE);
			write_int(scroll_slot);
			write_int(1);
			write_time();
		}
	};
}
