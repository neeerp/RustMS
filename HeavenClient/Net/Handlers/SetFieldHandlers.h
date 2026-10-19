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

	// A clock (v83 CField::OnClock 0x5361bd): kind 1 the time on a
	// station's wall clock; kinds 0, 2 and 3 a countdown at the top of the
	// screen, an event's or the field's.
	class ClockHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Takes the field's countdown down (v83 0x53184a).
	class StopClockHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A named map obj, such as a party quest's gate or trap, switches state
	// (v83 0x53300b for FIELD_OBSTACLE_ONOFF, 0x537a1e for SET_OBJECT_STATE).
	class ObjectStateHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Several named map objs switch state (v83 0x533057).
	class ObjectStateListHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Map objs with a tag are shown or hidden, such as the arrows to a job
	// instructor (v83 FUN_006449d2).
	class MapObjectVisibleHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Every named trap goes back to its first state (v83 0x5330b6).
	class ObstacleResetHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A ship field's ship comes or goes (v83 FUN_0054dca3).
	class ContiMoveHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Where a ship field's ship is, as the client asked (v83 FUN_0054dd08).
	class ContiStateHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
