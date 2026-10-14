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

#include "MovementPacket.h"

namespace ms
{
	// Opcode: CHANGE_CHANNEL(39)
	class ChangeChannelPacket : public OutPacket
	{
	public:
		explicit ChangeChannelPacket(uint8_t channel) : OutPacket(39)
		{
			write_byte(channel);
			write_int(0);
		}
	};

	// Opcode: CHANGE_MAP(38)
	// Tells the server the player touched a script portal (e.g. a tutorial hint)
	// Opcode: CHANGE_MAP_SPECIAL(100)
	class ChangeMapSpecialPacket : public OutPacket
	{
	public:
		ChangeMapSpecialPacket(const std::string &portal) : OutPacket(OutPacket::Opcode::CHANGE_MAP_SPECIAL)
		{
			write_byte(0);
			write_string(portal);
			write_short(0);
		}
	};

	class ChangeMapPacket : public OutPacket
	{
	public:
		// Request the server to warp the player to a different map
		ChangeMapPacket(bool died, int32_t targetid, const std::string& targetp, bool usewheel) : OutPacket(OutPacket::Opcode::CHANGEMAP)
		{
			write_byte(died);
			write_int(targetid);
			write_string(targetp);
			skip(1);
			write_short(usewheel ? 1 : 0);
		}

		// Request the server to exit the cash shop
		ChangeMapPacket() : OutPacket(OutPacket::Opcode::CHANGEMAP) {}
	};

	// Opcode: ENTER_CASHSHOP(40)
	class EnterCashShopPacket : public OutPacket
	{
	public:
		// Requests the server to warp the player into the cash shop
		EnterCashShopPacket() : OutPacket(OutPacket::Opcode::ENTER_CASHSHOP) {}
	};

	// Opcode: MOVE_PLAYER(41)
	class MovePlayerPacket : public MovementPacket
	{
	public:
		// Updates the player's position with the server
		MovePlayerPacket(const Movement& movement) : MovementPacket(OutPacket::Opcode::MOVE_PLAYER)
		{
			skip(9);
			write_byte(1);
			writemovement(movement);
		}
	};

	// Opcode: PARTY_OPERATION(124)
	class PartyOperationPacket : public OutPacket
	{
	public:
		enum Operation : int8_t
		{
			CREATE = 1,
			LEAVE = 2,
			JOIN = 3,
			INVITE = 4,
			EXPEL = 5,
			PASS_LEADER = 6
		};

	protected:
		PartyOperationPacket(Operation op) : OutPacket(OutPacket::Opcode::PARTY_OPERATION)
		{
			write_byte(op);
		}
	};

	// Creates a new party
	class CreatePartyPacket : public PartyOperationPacket
	{
	public:
		CreatePartyPacket() : PartyOperationPacket(PartyOperationPacket::Operation::CREATE) {}
	};

	// Leaves a party
	class LeavePartyPacket : public PartyOperationPacket
	{
	public:
		LeavePartyPacket() : PartyOperationPacket(PartyOperationPacket::Operation::LEAVE) {}
	};

	// Joins a party
	class JoinPartyPacket : public PartyOperationPacket
	{
	public:
		JoinPartyPacket(int32_t party_id) : PartyOperationPacket(PartyOperationPacket::Operation::JOIN)
		{
			write_int(party_id);
		}
	};

	// Invites a player to a party
	class InviteToPartyPacket : public PartyOperationPacket
	{
	public:
		InviteToPartyPacket(const std::string& name) : PartyOperationPacket(PartyOperationPacket::Operation::INVITE)
		{
			write_string(name);
		}
	};

	// Expels someone from a party
	class ExpelFromPartyPacket : public PartyOperationPacket
	{
	public:
		ExpelFromPartyPacket(int32_t cid) : PartyOperationPacket(PartyOperationPacket::Operation::EXPEL)
		{
			write_int(cid);
		}
	};

	// Passes party leadership to another character
	class ChangePartyLeaderPacket : public PartyOperationPacket
	{
	public:
		ChangePartyLeaderPacket(int32_t cid) : PartyOperationPacket(PartyOperationPacket::Operation::PASS_LEADER)
		{
			write_int(cid);
		}
	};

	// Turns down a party invitation
	// Opcode: DENY_PARTY_REQUEST(125)
	class DenyPartyRequestPacket : public OutPacket
	{
	public:
		static constexpr int8_t DENIED = 0x17;

		DenyPartyRequestPacket(const std::string& inviter) : OutPacket(OutPacket::Opcode::DENY_PARTY_REQUEST)
		{
			write_byte(DENIED);
			write_string(inviter);
		}
	};

	// Opcode: BUDDYLIST_MODIFY(130)
	class BuddyListModifyPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			ADD = 1,
			ACCEPT = 2,
			REMOVE = 3
		};

	protected:
		BuddyListModifyPacket(Mode mode) : OutPacket(OutPacket::Opcode::BUDDYLIST_MODIFY)
		{
			write_byte(mode);
		}
	};

	// Adds a character to the buddy list, asking them to add the player back
	class AddBuddyPacket : public BuddyListModifyPacket
	{
	public:
		AddBuddyPacket(const std::string& name, const std::string& group) : BuddyListModifyPacket(BuddyListModifyPacket::Mode::ADD)
		{
			write_string(name);
			write_string(group);
		}
	};

	// Accepts a buddy request
	class AcceptBuddyPacket : public BuddyListModifyPacket
	{
	public:
		AcceptBuddyPacket(int32_t cid) : BuddyListModifyPacket(BuddyListModifyPacket::Mode::ACCEPT)
		{
			write_int(cid);
		}
	};

	// Removes a buddy, or declines their request
	class DeleteBuddyPacket : public BuddyListModifyPacket
	{
	public:
		DeleteBuddyPacket(int32_t cid) : BuddyListModifyPacket(BuddyListModifyPacket::Mode::REMOVE)
		{
			write_int(cid);
		}
	};

	// Opcode: ADMIN_COMMAND(128)
	class AdminCommandPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			ENTER_MAP = 0x11
		};

	protected:
		AdminCommandPacket(Mode mode) : OutPacket(OutPacket::Opcode::ADMIN_COMMAND)
		{
			write_byte(mode);
		}
	};

	// Admin has entered the map
	class AdminEnterMapPacket : public AdminCommandPacket
	{
	public:
		enum Operation : int8_t
		{
			SHOW_USERS,
			ALERT_ADMINS = 12
		};

		AdminEnterMapPacket(Operation op) : AdminCommandPacket(AdminCommandPacket::Mode::ENTER_MAP)
		{
			write_byte(op);
		}
	};

	// Opcode: MOVE_MONSTER(188)
	class MoveMobPacket : public MovementPacket
	{
	public:
		// Updates a mob's position with the server
		MoveMobPacket(int32_t oid, int16_t type, int8_t skillb, int8_t skill0, int8_t skill1, int8_t skill2, int8_t skill3, int8_t skill4, Point<int16_t> startpos, const Movement& movement) : MovementPacket(OutPacket::Opcode::MOVE_MONSTER)
		{
			write_int(oid);
			write_short(type);
			write_byte(skillb);
			write_byte(skill0);
			write_byte(skill1);
			write_byte(skill2);
			write_byte(skill3);
			write_byte(skill4);

			skip(13);

			write_point(startpos);

			write_byte(1);
			writemovement(movement);
		}
	};

	// Opcode: PICKUP_ITEM(202)
	class PickupItemPacket : public OutPacket
	{
	public:
		// Requests picking up an item
		PickupItemPacket(int32_t oid, Point<int16_t> position) : OutPacket(OutPacket::Opcode::PICKUP_ITEM)
		{
			write_int(0);
			write_byte(0);
			write_point(position);
			write_int(oid);
		}
	};

	// Opcode: DAMAGE_REACTOR(205)
	class DamageReactorPacket : public OutPacket
	{
	public:
		// Requests damaging a reactor
		DamageReactorPacket(int32_t oid, Point<int16_t> position, int16_t stance, int skillid) : OutPacket(OutPacket::Opcode::DAMAGE_REACTOR)
		{
			write_int(oid);
			write_point(position);
			write_short(stance);
			skip(4);
			write_int(skillid);
		};
	};

	// Opcode: PLAYER_MAP_TRANSFER(207)
	class PlayerMapTransferPacket : public OutPacket
	{
	public:
		// Requests the server to set map transition complete
		PlayerMapTransferPacket() : OutPacket(OutPacket::Opcode::PLAYER_MAP_TRANSFER) {}
	};

	// Opcode: PLAYER_UPDATE(223)
	class PlayerUpdatePacket : public OutPacket
	{
	public:
		// Finished updating player stats
		PlayerUpdatePacket() : OutPacket(OutPacket::Opcode::PLAYER_UPDATE) {}
	};
}
