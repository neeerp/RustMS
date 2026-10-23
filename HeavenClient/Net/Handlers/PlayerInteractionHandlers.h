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
	// Handler for a packet which contains character info
	class CharInfoHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// What giving fame came to (v83 FUN_00a223dc): a line in chat, and for
	// the giver, the new fame in the open info window.
	class FameResponseHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The mini rooms (v83 FUN_0065df4c): invites, entering and leaving a
	// room, its chat, and the room's own modes. Only the trading room is
	// supported; other rooms are dropped.
	class PlayerInteractionHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Whether the hired merchant permit asked about may be used (0xa27d75).
	// Opcode: ENTRUSTED_SHOP_CHECK_RESULT(50)
	class EntrustedShopCheckResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};
}