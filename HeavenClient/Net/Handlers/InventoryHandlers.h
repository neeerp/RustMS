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

#include "../PacketHandler.h"

namespace ms
{
	// Handler for a packet which signifies that inventory items were gathered.
	class GatherResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handler for a packet which signifies that inventory items were sorted.
	class SortResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handler for a packet which gives an inventory tab more slots (v83
	// FUN_00a1f881): type, then the new slot count.
	class InventoryGrowHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// What came of keeping a map for a teleport rock or using one (v83
	// FUN_00a25268): the new list, shown in the open window, or a notice
	// for a refusal.
	class MapTransferResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// What came of Vicious' Hammer (v83 FUN_0082b2c3): 0x39 with a flag and
	// the hammers the equip has had, then 0x3d once the window has shown it,
	// or 0x3e with why not.
	// Opcode: VICIOUS_HAMMER(354)
	class ViciousHammerHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// What the Item Maker made (v83 FUN_0095dad3): a code, and for 0 or 1
	// what was made, gained and used up, as grey chat lines; then the
	// window's dialog.
	// Opcode: MAKER_RESULT(217)
	class MakerResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Vega's Spell's roll (FUN_0082d8d5): one code, first the one that
	// starts it and says how it went, then the one that lets it end.
	// Opcode: VEGA_SPELL(358)
	class VegaSpellHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// What a Pigmy Egg held, or nothing when the inventory was full (v83
	// 0xa28298). It does not unlock the client.
	// Opcode: INCUBATOR_RESULT(69)
	class IncubatorResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The mesos a meso bag gave, in an OK box (v83 FUN_009599bb). It does
	// not unlock the client.
	// Opcode: MESO_BAG_USED(209)
	class MesoBagUsedHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A meso bag that would not open, in an OK box (v83 FUN_00959a47).
	// Opcode: MESO_BAG_FAILED(210)
	class MesoBagFailedHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A summoning sack the server won't let open on this map: a 0 says so
	// in an OK box (v83 FUN_00532fcf).
	// Opcode: SUMMON_ITEM_INAVAILABLE(137)
	class SummonItemUnavailableHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The Owl of Minerva's answers: the items searched for most, for its
	// search window, or a search's rows (FUN_00a28c29).
	// Opcode: OWL_RESULT(70)
	class OwlResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Whether GO on a search row may go (FUN_00a29013, FUN_008a4e7a).
	// Opcode: OWL_GO_RESULT(71)
	class OwlGoResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The answer to a Maplemas Party Potion (FUN_00a2b39a): it unlocks the
	// client, and may say in the chat log why no one was transformed.
	// Opcode: RANDOM_MORPH_RES(119)
	class RandomMorphResHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The Lie Detector's news (FUN_00a265c2, report c025): a test begun, a
	// picture to read, how it ended or why it couldn't begin. The GM
	// skill's (code 2) is left out.
	// Opcode: ANTI_MACRO_RESULT(43)
	class AntiMacroResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Whether the Equip window has the second pendant cell (0xa13c7b). v83
	// reads it only when it builds the window (report b178).
	// Opcode: EXTRA_PENDANT_SLOT(121)
	class ExtraPendantSlotHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handler for a packet which modifies the player's inventory.
	class ModifyInventoryHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};
}