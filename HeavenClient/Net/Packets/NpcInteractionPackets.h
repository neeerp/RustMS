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

#include <string>

#include <cstdint>

#include "../OutPacket.h"

namespace ms
{
	// Packet which requests a dialog with a server-sided NPC
	// Opcode: TALK_TO_NPC(58)
	class TalkToNPCPacket : public OutPacket
	{
	public:
		TalkToNPCPacket(int32_t oid) : OutPacket(OutPacket::Opcode::TALK_TO_NPC)
		{
			write_int(oid);
		}
	};

	// The idle action the NPCs' controller chose for one (FUN_006d3273): the
	// action as an animation index from 2, or -1 for a line alone, then the
	// line, -1 for none. The server shows it to everyone.
	// Opcode: NPC_ACTION(197)
	class NpcActionPacket : public OutPacket
	{
	public:
		NpcActionPacket(int32_t oid, int8_t action, int8_t line) : OutPacket(OutPacket::Opcode::NPC_ACTION)
		{
			write_int(oid);
			write_byte(action);
			write_byte(line);
		}
	};

	// Packet which sends a response to an NPC dialog to the server
	// Opcode: NPC_TALK_MORE(60)
	class NpcTalkMorePacket : public OutPacket
	{
	public:
		NpcTalkMorePacket(int8_t lastmsg, int8_t response) : OutPacket(OutPacket::Opcode::NPC_TALK_MORE)
		{
			write_byte(lastmsg);
			write_byte(response);
		}

		// Answers a text (2) or box text (13) prompt with what was typed.
		NpcTalkMorePacket(int8_t lastmsg, int8_t response, const std::string& answer) : NpcTalkMorePacket(lastmsg, response)
		{
			write_string(answer);
		}

		// Answers a number prompt (3) with the number, or a menu (4) with the choice.
		NpcTalkMorePacket(int8_t lastmsg, int8_t response, int32_t answer) : NpcTalkMorePacket(lastmsg, response)
		{
			write_int(answer);
		}
	};

	// Packet which tells the server of an interaction with an NPC shop
	// Opcode: NPC_SHOP_ACTION(61)
	class NpcShopActionPacket : public OutPacket
	{
	public:
		// Requests that an item should be bought from or sold to a NPC shop
		NpcShopActionPacket(int16_t slot, int32_t itemid, int16_t qty, bool buy) : NpcShopActionPacket(buy ? Mode::BUY : Mode::SELL)
		{
			write_short(slot);
			write_int(itemid);
			write_short(qty);
		}

		// Requests that an item should be recharged at a NPC shop
		NpcShopActionPacket(int16_t slot) : NpcShopActionPacket(Mode::RECHARGE)
		{
			write_short(slot);
		}

		// Requests exiting from a NPC shop
		NpcShopActionPacket() : NpcShopActionPacket(Mode::LEAVE) {}

	protected:
		enum Mode : int8_t
		{
			BUY, SELL, RECHARGE, LEAVE
		};

		NpcShopActionPacket(Mode mode) : OutPacket(OutPacket::Opcode::NPC_SHOP_ACTION)
		{
			write_byte(mode);
		}
	};

	// Packet which asks an NPC's storage to take out, store, sort, or move
	// mesos, or closes it
	// Opcode: TRUNK_REQUEST(62)
	class TrunkRequestPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			TAKE_OUT = 4,
			STORE,
			SORT,
			MESO,
			CLOSE
		};

		// Sorts or closes the storage
		TrunkRequestPacket(Mode mode) : OutPacket(OutPacket::Opcode::TRUNK_REQUEST)
		{
			write_byte(mode);
		}

		// Takes out the index-th item of the storage's bag of a type
		TrunkRequestPacket(int8_t type, int8_t index) : TrunkRequestPacket(Mode::TAKE_OUT)
		{
			write_byte(type);
			write_byte(index);
		}

		// Stores count of the item in an inventory slot
		TrunkRequestPacket(int16_t slot, int32_t itemid, int16_t count) : TrunkRequestPacket(Mode::STORE)
		{
			write_short(slot);
			write_int(itemid);
			write_short(count);
		}

		// Takes mesos out, or puts them in when negative
		TrunkRequestPacket(int32_t meso) : TrunkRequestPacket(Mode::MESO)
		{
			write_int(meso);
		}
	};
}
