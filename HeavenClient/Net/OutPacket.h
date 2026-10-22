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

#include <cstddef>

#include <cstdint>

#include "../Template/Point.h"

#include <vector>

#include "stdio.h"

namespace ms
{
	// A packet to be sent to the server
	// Used as a base class to create specific packets
	class OutPacket
	{
	public:
		// Construct a packet by writing its opcode
		OutPacket(int16_t opcode);

		void dispatch();

		// Opcodes for OutPackets associated with version 83 of the game
		enum Opcode : uint16_t
		{
			/// Login
			LOGIN = 1,
			CHARLIST_REQUEST = 5,
			SERVERSTATUS_REQUEST = 6,
			ACCEPT_TOS = 7,
			SET_GENDER = 8,
			SERVERLIST_REQUEST = 11,
			SELECT_CHAR = 19,
			PLAYER_LOGIN = 20,
			NAME_CHAR = 21,
			CREATE_CHAR = 22,
			DELETE_CHAR = 23,
			PONG = 24,
			REGISTER_PIC = 29,
			SELECT_CHAR_PIC = 30,
			LOGIN_START = 35, // Custom name

			/// Gameplay 1
			CHANGEMAP = 38,
			ENTER_CASHSHOP = 40,
			MOVE_PLAYER = 41,
			CANCEL_CHAIR = 42,
			USE_CHAIR = 43,
			CLOSE_ATTACK = 44,
			RANGED_ATTACK = 45,
			MAGIC_ATTACK = 46,
			// Energy Charge's body attack on a touching mob (FUN_0095f135)
			TOUCH_MONSTER_ATTACK = 47,
			TAKE_DAMAGE = 48,
			USE_DEATHITEM = 53,

			/// Messaging
			GENERAL_CHAT = 49,
			CLOSE_CHALKBOARD = 50,
			MULTICHAT = 119,
			FACE_EXPRESSION = 51,
			USE_ITEMEFFECT = 52,
			WHISPER = 120,
			MESSENGER = 122,

			/// Monster Book
			MONSTER_BOOK_COVER = 57,

			/// NPC Interaction
			TALK_TO_NPC = 58,
			NPC_TALK_MORE = 60,
			NPC_SHOP_ACTION = 61,
			TRUNK_REQUEST = 62,
			DUEY_ACTION = 65,
			OWL_OPEN = 66,
			OWL_GO = 67,
			RPS_ACTION = 136,
			RING_ACTION = 137,

			/// Player Interaction
			CHAR_INFO_REQUEST = 97,
			SPAWN_PET = 98,
			GIVE_FAME = 95,
			PLAYER_INTERACTION = 123,

			/// Inventory
			GATHER_ITEMS = 69,
			SORT_ITEMS = 70,
			MOVE_ITEM = 71,
			USE_ITEM = 72,
			CANCEL_ITEM_EFFECT = 73,
			USE_SUMMON_BAG = 75,
			PET_FOOD = 76,
			USE_MOUNT_FOOD = 77,
			USE_CASH_ITEM = 79,
			DEAD_PET = 80,
			USE_CATCH_ITEM = 81,
			USE_SKILL_BOOK = 82,
			USE_OWL = 83,
			USE_TELEPORT_ROCK = 84,
			USE_RETURN_SCROLL = 85,
			MESO_DROP = 94,
			HEAL_OVER_TIME = 89,
			SCROLL_EQUIP = 86,

			/// Player
			SPEND_AP = 87,
			SPEND_SP = 90,
			CHANGE_KEYMAP = 135,

			/// Skill
			USE_SKILL = 91,
			CANCEL_BUFF = 92,
			SKILL_EFFECT = 93,

			/// Gameplay 2
			CHANGE_MAP_SPECIAL = 100,
			TELEPORT_ROCK_LIST = 102,
			QUEST_ACTION = 107,
			CALC_DAMAGE_STAT = 108,
			THROW_GRENADE = 109,
			SKILL_MACRO = 110,
			USE_ITEM_REWARD = 112,
			ITEM_MAKE = 113,
			WATER_OF_LIFE = 117,
			PARTY_OPERATION = 124,
			DENY_PARTY_REQUEST = 125,
			GUILD_OPERATION = 126,
			DENY_GUILD_REQUEST = 127,
			ADMIN_COMMAND = 128,
			BUDDYLIST_MODIFY = 130,
			NOTE_ACTION = 131,
			USE_DOOR = 133,
			ALLIANCE_OPERATION = 143,
			DENY_ALLIANCE_REQUEST = 144,
			OPEN_FAMILY_PEDIGREE = 145,
			OPEN_FAMILY = 146,
			ADD_FAMILY = 147,
			SEPARATE_FAMILY_BY_SENIOR = 148,
			SEPARATE_FAMILY_BY_JUNIOR = 149,
			ACCEPT_FAMILY = 150,
			USE_FAMILY = 151,
			CHANGE_FAMILY_MESSAGE = 152,
			FAMILY_SUMMON_RESPONSE = 153,
			GUILD_BBS = 155,
			USE_SOLOMON_ITEM = 157,
			USE_GACHA_EXP = 158,
			USE_TARGET_ITEM = 160,
			MOVE_PET = 167,
			PET_CHAT = 168,
			PET_COMMAND = 169,
			PET_LOOT = 170,
			PET_AUTO_POT = 171,
			PET_EXCLUDE_ITEMS = 172,
			MOVE_SUMMON = 175,
			SUMMON_ATTACK = 176,
			DAMAGE_SUMMON = 177,
			BEHOLDER = 178,
			QUICK_SLOT = 183,
			MOVE_MONSTER = 188,
			MOB_DISTANCE = 189,
			MOB_DAMAGE_MOB_FRIENDLY = 192,
			MOB_BOMB = 193,
			MOB_DAMAGE_MOB = 194,
			NPC_ACTION = 197,
			PICKUP_ITEM = 202,
			DAMAGE_REACTOR = 205,
			PLAYER_MAP_TRANSFER = 207,
			CONTI_STATE_REQUEST = 220,
			PARTY_SEARCH_START = 222,
			PARTY_SEARCH_STOP = 223,
			CHECK_CASH = 228,
			CASHSHOP_OPERATION = 229,
			USE_HAMMER = 260
		};

	protected:
		// Skip a number of bytes (filled with zeros)
		void skip(size_t count);
		// Write a byte
		void write_byte(int8_t ch);
		// Write a short
		void write_short(int16_t sh);
		// Write an int
		void write_int(int32_t in);
		// Write a long
		void write_long(int64_t lg);

		// Write a point
		// One short for x and one for y
		void write_point(Point<int16_t> point);
		// Write a timestamp as an integer
		void write_time();
		// Write a string
		// Writes the length as a short and then each individual character as a byte
		void write_string(const std::string& str);
		// Write a random int
		void write_random();

		// Write the MACS and then write the HWID of the computer
		void write_hardware_info();
		// Function to convert hexadecimal to decimal
		int32_t hex_to_dec(std::string hexVal);

	private:
		std::vector<int8_t> bytes;
		int16_t opcode;
	};
}
