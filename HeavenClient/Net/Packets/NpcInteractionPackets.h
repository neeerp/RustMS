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

	// Answers a quiz (5) or speed quiz (6) with what was typed, and no response byte (v83 7eb276, 7ec934).
	// Opcode: NPC_TALK_MORE(60)
	class NpcQuizAnswerPacket : public OutPacket
	{
	public:
		NpcQuizAnswerPacket(int8_t msgtype, const std::string& answer) : OutPacket(OutPacket::Opcode::NPC_TALK_MORE)
		{
			write_byte(msgtype);
			write_string(answer);
		}
	};

	// Answers a choice of styles (7 or 8) with the index of the one picked.
	// Opcode: NPC_TALK_MORE(60)
	class NpcTalkStylePacket : public NpcTalkMorePacket
	{
	public:
		NpcTalkStylePacket(int8_t lastmsg, uint8_t index) : NpcTalkMorePacket(lastmsg, 1)
		{
			write_byte(static_cast<int8_t>(index));
		}
	};

	// Answers a choice of pets (9 or 10) with the Cash Shop serial of the one picked.
	// Opcode: NPC_TALK_MORE(60)
	class NpcTalkPetPacket : public NpcTalkMorePacket
	{
	public:
		NpcTalkPetPacket(int8_t lastmsg, int64_t serial) : NpcTalkMorePacket(lastmsg, 1)
		{
			write_long(serial);
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

	// The Store Bank's Get All, its confirm, and its closing
	// Opcode: STORE_BANK_REQUEST(64)
	class StoreBankRequestPacket : public OutPacket
	{
	public:
		StoreBankRequestPacket(int8_t mode) : OutPacket(OutPacket::Opcode::STORE_BANK_REQUEST)
		{
			write_byte(mode);
		}
	};

	// A press in the Rock, Paper, Scissors window (v83 0x7403d0 to 0x7405a0)
	// Opcode: RPS_ACTION(136)
	class RpsActionPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			START = 0,
			CHOOSE = 1,
			TIME_UP = 2,
			CONTINUE = 3,
			EXIT = 4,
			RETRY = 5
		};

		RpsActionPacket(Mode mode) : OutPacket(OutPacket::Opcode::RPS_ACTION)
		{
			write_byte(mode);
		}

		// A hand: 0 rock, 1 paper, 2 scissors
		RpsActionPacket(int8_t hand) : OutPacket(OutPacket::Opcode::RPS_ACTION)
		{
			write_byte(Mode::CHOOSE);
			write_byte(hand);
		}
	};

	// Opcode: DUEY_ACTION(65)
	class DueyActionPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			OPEN = 0,
			SEND = 2,
			CLAIM = 4,
			REMOVE = 5,
			CLOSE = 7
		};

		// Opens or closes the window
		DueyActionPacket(Mode mode) : OutPacket(OutPacket::Opcode::DUEY_ACTION)
		{
			write_byte(mode);

			// The notifier's open request, as v83 sends it (FUN_00522585).
			if (mode == Mode::OPEN)
			{
				write_int(-1);
				write_int(2);
			}
		}

		// Claims or deletes a parcel
		DueyActionPacket(Mode mode, int32_t id) : OutPacket(OutPacket::Opcode::DUEY_ACTION)
		{
			write_byte(mode);
			write_int(id);
		}

		// Sends an item from an inventory slot, if any, and mesos (FUN_006f36a8)
		DueyActionPacket(int8_t tab, int16_t slot, int16_t count, int32_t meso, const std::string& to)
			: OutPacket(OutPacket::Opcode::DUEY_ACTION)
		{
			write_sent(tab, slot, count, meso, to);
			write_byte(0);
		}

		// The same as a quick delivery, with a message and the coupon's Cash
		// slot (FUN_006f1df5)
		DueyActionPacket(int8_t tab, int16_t slot, int16_t count, int32_t meso, const std::string& to,
			const std::string& message, int32_t coupon) : OutPacket(OutPacket::Opcode::DUEY_ACTION)
		{
			write_sent(tab, slot, count, meso, to);
			write_byte(1);
			write_string(message);
			write_int(coupon);
		}

	private:
		void write_sent(int8_t tab, int16_t slot, int16_t count, int32_t meso, const std::string& to)
		{
			write_byte(Mode::SEND);
			write_byte(tab);
			write_short(slot);
			write_short(count);
			write_int(meso);
			write_string(to);
		}
	};
}
