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
#include <vector>

#include "../OutPacket.h"

namespace ms
{
	// Packet which sends a message to general chat.
	// Opcode: GENERAL_CHAT(49)
	class GeneralChatPacket : public OutPacket
	{
	public:
		GeneralChatPacket(const std::string& message, bool show) : OutPacket(OutPacket::Opcode::GENERAL_CHAT)
		{
			write_string(message);
			write_byte(show);
		}
	};

	// Sends a line to the player's party. The server decides who hears it,
	// but the client lists the recipients.
	// Opcode: MULTICHAT(119)
	class PartyChatPacket : public OutPacket
	{
	public:
		static constexpr int8_t PARTY_MODE = 1;

		PartyChatPacket(const std::vector<int32_t>& recipients, const std::string& message) : OutPacket(OutPacket::Opcode::MULTICHAT)
		{
			write_byte(PARTY_MODE);
			write_byte(static_cast<int8_t>(recipients.size()));

			for (int32_t cid : recipients)
				write_int(cid);

			write_string(message);
		}
	};

	// Packet which sends a whisper/private message to another player.
	// Opcode: WHISPER(120)
	class WhisperPacket : public OutPacket
	{
	public:
		static constexpr int8_t REQUEST_MODE = 0x06;

		WhisperPacket(const std::string& target, const std::string& message) : OutPacket(OutPacket::Opcode::WHISPER)
		{
			write_byte(REQUEST_MODE);
			write_string(target);
			write_string(message);
		}
	};
}
