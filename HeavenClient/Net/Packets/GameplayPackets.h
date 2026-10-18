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

#include <cstdint>

#include "MovementPacket.h"

#include "../../Gameplay/MovePath.h"

#include "../../Character/Inventory/InventoryType.h"

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

	// Tells the server the player entered a script portal (e.g. a tutorial
	// hint), as v83 does from FUN_0094dac6 and FUN_0094df9b
	// Opcode: CHANGE_MAP_SPECIAL(100)
	class ChangeMapSpecialPacket : public OutPacket
	{
	public:
		ChangeMapSpecialPacket(uint8_t fieldkey, const std::string &portal, Point<int16_t> position) :
				OutPacket(OutPacket::Opcode::CHANGE_MAP_SPECIAL)
		{
			write_byte(fieldkey);
			write_string(portal);
			write_point(position);
		}
	};

	// Go through a Mystic Door: in a field, to town; in town, back to its
	// field (FUN_007bd594, FUN_005375ed).
	// Opcode: USE_DOOR(133)
	class UseDoorPacket : public OutPacket
	{
	public:
		UseDoorPacket(int32_t owner, bool from_town) : OutPacket(OutPacket::Opcode::USE_DOOR)
		{
			write_int(owner);
			write_byte(from_town);
		}
	};

	// Opcode: CHANGE_MAP(38)
	class ChangeMapPacket : public OutPacket
	{
	public:
		// Request the server to warp the player to a different map. v83
		// starts every request with the field's key (FUN_0053035d), dead
		// or alive.
		ChangeMapPacket(uint8_t fieldkey, int32_t targetid, const std::string& targetp, bool usewheel) : OutPacket(OutPacket::Opcode::CHANGEMAP)
		{
			write_byte(fieldkey);
			write_int(targetid);
			write_string(targetp);
			skip(1);
			write_short(usewheel ? 1 : 0);
		}

		// Ask to go through a portal, from where the player stands (v83
		// FUN_0053035d)
		ChangeMapPacket(uint8_t fieldkey, const std::string &portal, Point<int16_t> position) : OutPacket(OutPacket::Opcode::CHANGEMAP)
		{
			write_byte(fieldkey);
			write_int(-1);
			write_string(portal);
			write_point(position);
			skip(1);
			write_short(0);
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

	// Opcode: CASHSHOP_OPERATION(229)
	class BuyCashItemPacket : public OutPacket
	{
	public:
		// Pay for the commodity sn: cash type 1 is NX Credit, 2 Maple Points,
		// 4 NX Prepaid
		BuyCashItemPacket(int32_t cash_type, int32_t sn) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(3);
			write_byte(0);
			write_int(cash_type);
			write_int(sn);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class BuyCashPackagePacket : public OutPacket
	{
	public:
		// Pay for the package sn, whose items each join the Cash Inventory
		BuyCashPackagePacket(int32_t cash_type, int32_t sn) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(0x1E);
			write_byte(0);
			write_int(cash_type);
			write_int(sn);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class ExpandSlotsPacket : public OutPacket
	{
	public:
		// Four more slots in a bag, or in the storage with type NONE, for
		// 4,000 NX: the Cash Inventory's slot buttons
		ExpandSlotsPacket(int32_t cash_type, InventoryType::Id type) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_header(cash_type, type, 0);

			if (type != InventoryType::Id::NONE)
				write_byte(type);
		}

		// Eight more slots: the expansion commodity sn
		ExpandSlotsPacket(int32_t cash_type, InventoryType::Id type, int32_t sn) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_header(cash_type, type, 1);
			write_int(sn);
		}

	private:
		void write_header(int32_t cash_type, InventoryType::Id type, int8_t mode)
		{
			write_byte(type == InventoryType::Id::NONE ? 7 : 6);
			write_byte(0);
			write_int(cash_type);
			write_byte(mode);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class GiftCashItemPacket : public OutPacket
	{
	public:
		// Send the commodity sn to the character named to. The birthday code
		// is the date as YYYYMMDD.
		GiftCashItemPacket(int32_t birthday, int32_t sn, const std::string& to, const std::string& message) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(4);
			write_int(birthday);
			write_int(sn);
			write_string(to);
			write_string(message);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class SetWishListPacket : public OutPacket
	{
	public:
		// Save the wish list's ten slots; 0 marks an empty one
		SetWishListPacket(const std::vector<int32_t>& sns) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(5);

			for (size_t i = 0; i < 10; i++)
				write_int(i < sns.size() ? sns[i] : 0);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class TakeOutCashItemPacket : public OutPacket
	{
	public:
		// Move a Cash Inventory item to its bag, at slot if that is free
		TakeOutCashItemPacket(int64_t cash_id, InventoryType::Id type, int16_t slot) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(0x0D);
			write_long(cash_id);
			write_byte(type);
			write_short(slot);
		}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class PutBackCashItemPacket : public OutPacket
	{
	public:
		// Move a cash item from a bag back to the Cash Inventory
		PutBackCashItemPacket(int64_t cash_id, InventoryType::Id type) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(0x0E);
			write_long(cash_id);
			write_byte(type);
		}
	};

	// Opcode: MOVE_PLAYER(41)
	class MovePlayerPacket : public MovementPacket
	{
	public:
		// Updates the player's position with the server: the field key and
		// crc (left 0), then the path.
		MovePlayerPacket(const move_path::Path& path) : MovementPacket(OutPacket::Opcode::MOVE_PLAYER)
		{
			skip(5);
			writepath(path.start, path.elements);
		}
	};

	// Opcode: MOVE_SUMMON(175)
	class MoveSummonPacket : public MovementPacket
	{
	public:
		// The player's summon moved (FUN_009c84e9): its object id, then a
		// path as the player's.
		MoveSummonPacket(int32_t oid, const move_path::Path& path) : MovementPacket(OutPacket::Opcode::MOVE_SUMMON)
		{
			write_int(oid);
			writepath(path.start, path.elements);
		}
	};

	// Shows the player's emote to the rest of the map
	// Opcode: FACE_EXPRESSION(51)
	class FaceExpressionPacket : public OutPacket
	{
	public:
		FaceExpressionPacket(int32_t emote) : OutPacket(OutPacket::Opcode::FACE_EXPRESSION)
		{
			write_int(emote);
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
		// v83 follows the operation with a zero byte (FUN_0052fe00).
		LeavePartyPacket() : PartyOperationPacket(PartyOperationPacket::Operation::LEAVE)
		{
			write_byte(0);
		}
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

	// Opcode: GUILD_OPERATION(126)
	class GuildOperationPacket : public OutPacket
	{
	public:
		enum Operation : int8_t
		{
			CREATE = 2
		};

		GuildOperationPacket(Operation op) : OutPacket(OutPacket::Opcode::GUILD_OPERATION)
		{
			write_byte(op);
		}
	};

	// Names the guild being founded, in answer to the name prompt
	class CreateGuildPacket : public GuildOperationPacket
	{
	public:
		CreateGuildPacket(const std::string& name) : GuildOperationPacket(GuildOperationPacket::Operation::CREATE)
		{
			write_string(name);
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
		// Updates a mob's position with the server, as v83 does
		// (FUN_0066b6fc): its move id, flags, its action (-1 for none) and
		// facing, skill data, an anti-cheat byte and reason, and two ints a
		// flyer fills in but others leave at 0xFFDDCC, then the path.
		MoveMobPacket(int32_t oid, int16_t moveid, int8_t flags, int8_t action, bool left, int32_t skilldata, bool flying, const move_path::Path& path) : MovementPacket(OutPacket::Opcode::MOVE_MONSTER)
		{
			write_int(oid);
			write_short(moveid);
			write_byte(flags);
			write_byte(action < 0 ? -1 : (action << 1) | left);
			write_int(skilldata);
			write_byte(0);
			write_int(0);

			int32_t unknown = flying ? 0 : 0x00FFDDCC;
			write_int(unknown);
			write_int(unknown);

			writepath(path.start, path.elements);
		}
	};

	// Opcode: MOB_DISTANCE(189)
	class MobDistancePacket : public OutPacket
	{
	public:
		// Tells the server how far the player is from a mob
		MobDistancePacket(int32_t oid, int32_t distance) : OutPacket(OutPacket::Opcode::MOB_DISTANCE)
		{
			write_int(oid);
			write_int(distance);
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
