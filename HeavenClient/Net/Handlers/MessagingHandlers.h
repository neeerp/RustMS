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

#include "../PacketHandler.h"

#include "../../Graphics/Text.h"
#include "../../IO/UITypes/UIChatBar.h"

namespace ms
{
	// Show a status message
	// Opcode: SHOW_STATUS_INFO(39)
	class ShowStatusInfoHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	private:
		// A picked-up item's name is cut to these widths, for one item or
		// several.
		static constexpr size_t PICKUP_NAME_WIDTH = 120;
		static constexpr size_t PICKUP_STACK_NAME_WIDTH = 96;

		void show_status(Color::Name color, const std::string& message) const;
		// A line in the chat log, grey unless given another type.
		void chat_line(const std::string& message, UIChatBar::LineType type = UIChatBar::LineType::GREY) const;
	};

	// Show an instruction balloon above the player
	// Opcode: PLAYER_HINT(214)
	class PlayerHintHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show a server message
	// Opcode: SERVER_MESSAGE(68)
	class ServerMessageHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show another type of server message
	// Opcode: WEEK_EVENT_MESSAGE(77)
	class WeekEventMessageHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show a chat message
	// CHAT_RECEIVED(162)
	class ChatReceivedHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handle whisper result and incoming whisper delivery
	// Opcode: WHISPER(135)
	class WhisperHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Maple Messenger: members joining, leaving and talking, and invitations
	// Opcode: MESSENGER(313)
	class MessengerHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Shows the effect of a scroll
	// Opcode: SCROLL_RESULT(167)
	class ScrollResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Can contain numerous different effects and messages
	// Opcode: SHOW_ITEM_GAIN_INCHAT(206)
	class ShowItemGainInChatHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};
}
