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
#include <optional>

#include "../PacketHandler.h"
#include "../../Template/Point.h"

namespace ms
{
	// Handler for a packet which contains all character information on first login or warps the player to a different map
	class SetFieldHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;

	private:
		void transition(int32_t mapid, uint8_t portalid, std::optional<Point<int16_t>> chase = std::nullopt) const;
		void change_map(InPacket& recv, int32_t map_id) const;
		void set_field(InPacket& recv) const;
	};

	// A map change the server refused (v83 FUN_0053185c): the request is
	// answered, and the reason goes to chat or, for 4 and 7, a notice.
	class BlockedMapHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A channel, cash shop or trade shop move the server refused (v83
	// FUN_00531a08): the request is answered and the reason goes to chat.
	class BlockedServerHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
