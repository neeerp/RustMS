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
		// A balloon-only line isn't written to anyone's chat log. v83's chat
		// bar never sends one (it calls FUN_0052c315 with 0).
		GeneralChatPacket(const std::string& message, bool balloon_only) : OutPacket(OutPacket::Opcode::GENERAL_CHAT)
		{
			write_string(message);
			write_byte(balloon_only);
		}
	};

	// Sends a line to the player's party or buddies. The server decides who
	// hears it, but the client lists the recipients.
	// Opcode: MULTICHAT(119)
	class MultiChatPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			BUDDY = 0,
			PARTY = 1,
			GUILD = 2
		};

		MultiChatPacket(Mode mode, const std::vector<int32_t>& recipients, const std::string& message) : OutPacket(OutPacket::Opcode::MULTICHAT)
		{
			write_byte(mode);
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

	// Asks where a character is (/find)
	// Opcode: WHISPER(120)
	class FindPacket : public OutPacket
	{
	public:
		static constexpr int8_t REQUEST_MODE = 0x05;

		FindPacket(const std::string& target) : OutPacket(OutPacket::Opcode::WHISPER)
		{
			write_byte(REQUEST_MODE);
			write_string(target);
		}
	};
	// Maple Messenger requests. Every one starts with a mode byte.
	// Opcode: MESSENGER(122)
	class MessengerPacket : public OutPacket
	{
	protected:
		enum Mode : int8_t
		{
			OPEN = 0x00,
			LEAVE = 0x02,
			INVITE = 0x03,
			DECLINE = 0x05,
			CHAT = 0x06
		};

		MessengerPacket(Mode mode) : OutPacket(OutPacket::Opcode::MESSENGER)
		{
			write_byte(mode);
		}
	};

	// Opens a new messenger (id 0) or joins the one we were invited to.
	class MessengerOpenPacket : public MessengerPacket
	{
	public:
		MessengerOpenPacket(int32_t messenger_id) : MessengerPacket(Mode::OPEN)
		{
			write_int(messenger_id);
		}
	};

	class MessengerLeavePacket : public MessengerPacket
	{
	public:
		MessengerLeavePacket() : MessengerPacket(Mode::LEAVE) {}
	};

	class MessengerInvitePacket : public MessengerPacket
	{
	public:
		MessengerInvitePacket(const std::string& target) : MessengerPacket(Mode::INVITE)
		{
			write_string(target);
		}
	};

	// X on an invitation (FUN_0051fff5); v83 ends with 1 instead when its
	// options turn invitations away unseen (FUN_00851478).
	class MessengerDeclinePacket : public MessengerPacket
	{
	public:
		MessengerDeclinePacket(const std::string& inviter, const std::string& self) : MessengerPacket(Mode::DECLINE)
		{
			write_string(inviter);
			write_string(self);
			write_byte(0);
		}
	};

	// A chat line, formatted as "name : text".
	class MessengerChatPacket : public MessengerPacket
	{
	public:
		MessengerChatPacket(const std::string& line) : MessengerPacket(Mode::CHAT)
		{
			write_string(line);
		}
	};
}
