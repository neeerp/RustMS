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

#include <array>
#include <string>
#include <vector>

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <optional>

#include "MovementPacket.h"

#include "../../Gameplay/MovePath.h"

#include "../../Character/Guild.h"
#include "../../Character/Inventory/InventoryType.h"
#include "../../Util/Memos.h"

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

	// Tells the server which item the death dialog offers, and where the
	// tombstone fell (v83 FUN_0095af8e)
	// Opcode: USE_DEATHITEM(53)
	class UseDeathItemPacket : public OutPacket
	{
	public:
		UseDeathItemPacket(int32_t itemid, Point<int16_t> tomb) : OutPacket(OutPacket::Opcode::USE_DEATHITEM)
		{
			write_int(itemid);
			write_int(tomb.x());
			write_int(tomb.y());
		}
	};

	// Opcode: ENTER_CASHSHOP(40)
	class EnterCashShopPacket : public OutPacket
	{
	public:
		// Requests the server to warp the player into the cash shop
		EnterCashShopPacket() : OutPacket(OutPacket::Opcode::ENTER_CASHSHOP) {}
	};

	// Opcode: CHECK_CASH(228)
	class CheckCashPacket : public OutPacket
	{
	public:
		// Ask for the balances again: the Check button, and v83 whenever the
		// game window is activated in the Cash Shop
		CheckCashPacket() : OutPacket(OutPacket::Opcode::CHECK_CASH) {}
	};

	// Opcode: CASHSHOP_OPERATION(229)
	class BuyCashItemPacket : public OutPacket
	{
	public:
		// Pay for the commodity sn: cash type 1 is NX Credit, 2 Maple Points,
		// 4 NX Prepaid. v83 flags a Maple Points purchase before the type,
		// and ends with 1 for the free one-day trial, which isn't offered.
		BuyCashItemPacket(int32_t cash_type, int32_t sn) : OutPacket(OutPacket::Opcode::CASHSHOP_OPERATION)
		{
			write_byte(3);
			write_byte(cash_type == 2);
			write_int(cash_type);
			write_int(sn);
			write_int(0);
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
			write_byte(cash_type == 2);
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
			write_byte(cash_type == 2);
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

	// Opcode: NOTE_ACTION(131)
	class GiftNotePacket : public OutPacket
	{
	public:
		// A note back to whoever sent the gift with cash serial cash_id, the
		// index-th of those just received (v83 4796c6).
		GiftNotePacket(const std::string& to, const std::string& note, int32_t index, int64_t cash_id) : OutPacket(OutPacket::Opcode::NOTE_ACTION)
		{
			write_byte(0);
			write_string(to);
			write_string(note);
			write_byte(1);
			write_int(index);
			write_long(cash_id);
		}
	};

	// Opcode: NOTE_ACTION(131)
	class ReadNotesPacket : public OutPacket
	{
	public:
		// The inbox closed and these notes are read (v83 64aa57). Each
		// invitation brings its item, one per free Etc slot; those left
		// over aren't sent, though counted.
		ReadNotesPacket(const std::vector<Memos::Memo>& memos, int8_t free_etc) : OutPacket(OutPacket::Opcode::NOTE_ACTION)
		{
			int8_t invitations = static_cast<int8_t>(std::count_if(memos.begin(), memos.end(),
					[](const Memos::Memo& memo) { return memo.flag == Memos::INVITATION; }));

			write_byte(1);
			write_byte(static_cast<int8_t>(memos.size()));
			write_byte(invitations);
			write_byte(free_etc);

			for (const Memos::Memo& memo : memos)
			{
				if (memo.flag != Memos::INVITATION)
				{
					write_int(memo.id);
					write_byte(static_cast<int8_t>(memo.flag));
				}
				else if (free_etc > 0)
				{
					size_t mark = memo.text.find('_');

					write_int(memo.id);
					write_byte(static_cast<int8_t>(memo.flag));
					write_int(mark == std::string::npos ? 0 : std::atoi(memo.text.c_str() + mark + 1));
					free_etc--;
				}
			}
		}
	};

	// Opcode: NOTE_ACTION(131)
	class AskNotesPacket : public OutPacket
	{
	public:
		// A note arrived; the server answers with the list (v83 a251ef).
		AskNotesPacket() : OutPacket(OutPacket::Opcode::NOTE_ACTION)
		{
			write_byte(2);
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

	// Opcode: MOVE_PET(167)
	class MovePetPacket : public MovementPacket
	{
	public:
		// The player's pet moved (FUN_009c4e41): its cash serial, then a
		// path as the player's.
		MovePetPacket(int64_t serial, const move_path::Path& path) : MovementPacket(OutPacket::Opcode::MOVE_PET)
		{
			write_long(serial);
			writepath(path.start, path.elements);
		}
	};

	// Opcode: SPAWN_PET(98)
	class SpawnPetPacket : public OutPacket
	{
	public:
		// Bring out the pet in Cash slot `slot`, or send it back if it is
		// out (FUN_00a240a2); `boss` makes a new pet lead the others.
		SpawnPetPacket(int16_t slot, bool boss) : OutPacket(OutPacket::Opcode::SPAWN_PET)
		{
			write_time();
			write_short(slot);
			write_byte(boss);
		}
	};

	// Opcode: DEAD_PET(80)
	class DeadPetPacket : public OutPacket
	{
	public:
		// Sent for the doll `serial`, which can't be revived, as the Cash
		// Shop is offered instead (FUN_00a240a2).
		DeadPetPacket(int64_t serial) : OutPacket(OutPacket::Opcode::DEAD_PET)
		{
			write_time();
			write_long(serial);
		}
	};

	// Asks the server to revive a doll with the Water of Life just used
	// (FUN_00a1dce6); it carries nothing.
	// Opcode: WATER_OF_LIFE(117)
	class WaterOfLifePacket : public OutPacket
	{
	public:
		WaterOfLifePacket() : OutPacket(OutPacket::Opcode::WATER_OF_LIFE) {}
	};

	// Opcode: PET_CHAT(168)
	class PetChatPacket : public OutPacket
	{
	public:
		// What the player's pet did and said, for the others to see
		// (FUN_007055e2). Its base actions go as 0.
		PetChatPacket(int64_t serial, uint8_t type, int16_t action, const std::string& text)
			: OutPacket(OutPacket::Opcode::PET_CHAT)
		{
			write_long(serial);
			write_byte(type);
			write_byte(static_cast<int8_t>(action < 9 ? 0 : action));
			write_string(text);
		}
	};

	// Opcode: PET_COMMAND(169)
	class PetCommandPacket : public OutPacket
	{
	public:
		// The player gave their pet a command in chat (FUN_00704d5d),
		// `named` when they called it by name.
		PetCommandPacket(int64_t serial, bool named, uint8_t command) : OutPacket(OutPacket::Opcode::PET_COMMAND)
		{
			write_long(serial);
			write_byte(named);
			write_byte(command);
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

	// Shows the player's effect item to the rest of the map, or stops it with 0
	// Opcode: USE_ITEMEFFECT(52)
	class UseItemEffectPacket : public OutPacket
	{
	public:
		UseItemEffectPacket(int32_t item_id) : OutPacket(OutPacket::Opcode::USE_ITEMEFFECT)
		{
			write_int(item_id);
		}
	};

	// Asks to take the player's chalkboard down
	// Opcode: CLOSE_CHALKBOARD(50)
	class CloseChalkboardPacket : public OutPacket
	{
	public:
		CloseChalkboardPacket() : OutPacket(OutPacket::Opcode::CLOSE_CHALKBOARD) {}
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

	// Joins a party. v83 follows the id with the invitation's party-search
	// byte (FUN_00a3e31c, FUN_00522585).
	class JoinPartyPacket : public PartyOperationPacket
	{
	public:
		JoinPartyPacket(int32_t party_id, int8_t search = 0) : PartyOperationPacket(PartyOperationPacket::Operation::JOIN)
		{
			write_int(party_id);
			write_byte(search);
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

	// Turns down a party invitation: X on the invite (DENIED), one that came
	// while another was still showing (BUSY), or one the Game Option blocks
	// (BLOCKING) (FUN_00a3e31c, FUN_0051fff5).
	// Opcode: DENY_PARTY_REQUEST(125)
	class DenyPartyRequestPacket : public OutPacket
	{
	public:
		static constexpr int8_t BLOCKING = 0x15, BUSY = 0x16, DENIED = 0x17;

		DenyPartyRequestPacket(int8_t reason, const std::string& inviter, const std::string& self) :
			OutPacket(OutPacket::Opcode::DENY_PARTY_REQUEST)
		{
			write_byte(reason);
			write_string(inviter);
			write_string(self);
		}
	};

	// Opcode: GUILD_OPERATION(126)
	class GuildOperationPacket : public OutPacket
	{
	public:
		enum Operation : int8_t
		{
			RELOAD = 0,
			CREATE = 2,
			INVITE = 5,
			JOIN = 6,
			LEAVE = 7,
			EXPEL = 8,
			SET_TITLES = 13,
			CHANGE_RANK = 14,
			SET_EMBLEM = 15,
			SET_NOTICE = 16
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

	// Invites a character to the player's guild
	class InviteToGuildPacket : public GuildOperationPacket
	{
	public:
		InviteToGuildPacket(const std::string& name) : GuildOperationPacket(GuildOperationPacket::Operation::INVITE)
		{
			write_string(name);
		}
	};

	// Asks for the player's guild again, after joining one
	class ReloadGuildPacket : public GuildOperationPacket
	{
	public:
		ReloadGuildPacket() : GuildOperationPacket(GuildOperationPacket::Operation::RELOAD) {}
	};

	// Takes up an invitation to a guild
	class JoinGuildPacket : public GuildOperationPacket
	{
	public:
		JoinGuildPacket(int32_t guild_id, int32_t cid) : GuildOperationPacket(GuildOperationPacket::Operation::JOIN)
		{
			write_int(guild_id);
			write_int(cid);
		}
	};

	// Quits the player's guild
	class LeaveGuildPacket : public GuildOperationPacket
	{
	public:
		LeaveGuildPacket(int32_t cid, const std::string& name) :
			GuildOperationPacket(GuildOperationPacket::Operation::LEAVE)
		{
			write_int(cid);
			write_string(name);
		}
	};

	// Expels a member from the player's guild
	class ExpelFromGuildPacket : public GuildOperationPacket
	{
	public:
		ExpelFromGuildPacket(int32_t cid, const std::string& name) :
			GuildOperationPacket(GuildOperationPacket::Operation::EXPEL)
		{
			write_int(cid);
			write_string(name);
		}
	};

	// Saves the five rank titles, changed or not, from the rank panel
	class SetGuildTitlesPacket : public GuildOperationPacket
	{
	public:
		SetGuildTitlesPacket(const std::array<std::string, 5>& titles) :
			GuildOperationPacket(GuildOperationPacket::Operation::SET_TITLES)
		{
			for (auto& title : titles)
				write_string(title);
		}
	};

	// Moves a member one rank up or down
	class ChangeGuildRankPacket : public GuildOperationPacket
	{
	public:
		ChangeGuildRankPacket(int32_t cid, uint8_t rank) :
			GuildOperationPacket(GuildOperationPacket::Operation::CHANGE_RANK)
		{
			write_int(cid);
			write_byte(rank);
		}
	};

	// Saves the emblem made in Lea's emblem maker, with the full ids
	class SetGuildEmblemPacket : public GuildOperationPacket
	{
	public:
		SetGuildEmblemPacket(const GuildEmblem& emblem) :
			GuildOperationPacket(GuildOperationPacket::Operation::SET_EMBLEM)
		{
			write_short(emblem.background);
			write_byte(emblem.background_color);
			write_short(emblem.mark);
			write_byte(emblem.mark_color);
		}
	};

	// Sets the guild notice
	class SetGuildNoticePacket : public GuildOperationPacket
	{
	public:
		SetGuildNoticePacket(const std::string& notice) :
			GuildOperationPacket(GuildOperationPacket::Operation::SET_NOTICE)
		{
			write_string(notice);
		}
	};

	// Turns down an invitation to a guild; the inviter hears the reason
	// Opcode: DENY_GUILD_REQUEST(127)
	class DenyGuildRequestPacket : public OutPacket
	{
	public:
		// Not taking invitations, already considering another, or declined.
		static constexpr int8_t BUSY = 0x35, ANOTHER = 0x36, DECLINED = 0x37;

		DenyGuildRequestPacket(int8_t reason, const std::string& inviter, const std::string& self) :
			OutPacket(OutPacket::Opcode::DENY_GUILD_REQUEST)
		{
			write_byte(reason);
			write_string(inviter);
			write_string(self);
		}
	};

	// Opcode: ALLIANCE_OPERATION(143)
	class AllianceOperationPacket : public OutPacket
	{
	public:
		enum Operation : int8_t
		{
			INFO = 1,
			LEAVE = 2,
			INVITE = 3,
			ACCEPT = 4,
			EXPEL = 6,
			CHANGE_MASTER = 7,
			SET_TITLES = 8,
			CHANGE_RANK = 9,
			SET_NOTICE = 10
		};

		AllianceOperationPacket(Operation op) : OutPacket(OutPacket::Opcode::ALLIANCE_OPERATION)
		{
			write_byte(op);
		}
	};

	// Asks for the alliance and its guilds, after the guild's info or
	// joining one
	class AllianceInfoPacket : public AllianceOperationPacket
	{
	public:
		AllianceInfoPacket() : AllianceOperationPacket(AllianceOperationPacket::Operation::INFO) {}
	};

	// The Alliance tab's requests (FUN_00912b49's buttons, b025): a guild
	// master leaves with their guild
	class LeaveAlliancePacket : public AllianceOperationPacket
	{
	public:
		LeaveAlliancePacket() : AllianceOperationPacket(AllianceOperationPacket::Operation::LEAVE) {}
	};

	// The master invites a guild by name
	class InviteToAlliancePacket : public AllianceOperationPacket
	{
	public:
		InviteToAlliancePacket(const std::string& guild) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::INVITE)
		{
			write_string(guild);
		}
	};

	// The master expels a guild: its id, then the alliance's
	class ExpelFromAlliancePacket : public AllianceOperationPacket
	{
	public:
		ExpelFromAlliancePacket(int32_t guild_id, int32_t alliance_id) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::EXPEL)
		{
			write_int(guild_id);
			write_int(alliance_id);
		}
	};

	// The master hands the alliance to another guild's master
	class ChangeAllianceMasterPacket : public AllianceOperationPacket
	{
	public:
		ChangeAllianceMasterPacket(int32_t cid) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::CHANGE_MASTER)
		{
			write_int(cid);
		}
	};

	// The master's new titles for ranks 1 to 5
	class SetAllianceTitlesPacket : public AllianceOperationPacket
	{
	public:
		SetAllianceTitlesPacket(const std::array<std::string, 5>& titles) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::SET_TITLES)
		{
			for (const std::string& title : titles)
				write_string(title);
		}
	};

	// A member one rank up (true) or down
	class ChangeAllianceRankPacket : public AllianceOperationPacket
	{
	public:
		ChangeAllianceRankPacket(int32_t cid, bool up) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::CHANGE_RANK)
		{
			write_int(cid);
			write_byte(up ? 1 : 0);
		}
	};

	class SetAllianceNoticePacket : public AllianceOperationPacket
	{
	public:
		SetAllianceNoticePacket(const std::string& notice) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::SET_NOTICE)
		{
			write_string(notice);
		}
	};

	// Takes up an invitation to an alliance (FUN_00522585): its id and the
	// player's own guild's name
	class JoinAlliancePacket : public AllianceOperationPacket
	{
	public:
		JoinAlliancePacket(int32_t alliance_id, const std::string& guild) :
			AllianceOperationPacket(AllianceOperationPacket::Operation::ACCEPT)
		{
			write_int(alliance_id);
			write_string(guild);
		}
	};

	// Turns down an invitation to an alliance: the Game Option blocks it
	// (BLOCKING), another is still showing (ANOTHER) or X (DECLINED)
	// (FUN_00a39f4e, FUN_0051fff5).
	// Opcode: DENY_ALLIANCE_REQUEST(144)
	class DenyAllianceRequestPacket : public OutPacket
	{
	public:
		static constexpr int8_t BLOCKING = 0x14, ANOTHER = 0x15, DECLINED = 0x16;

		DenyAllianceRequestPacket(int8_t reason, const std::string& inviter, const std::string& guild) :
			OutPacket(OutPacket::Opcode::DENY_ALLIANCE_REQUEST)
		{
			write_byte(reason);
			write_string(inviter);
			write_string(guild);
		}
	};

	// A request to the guild bulletin board (0x9B)
	// Opcode: GUILD_BBS(155)
	class GuildBbsPacket : public OutPacket
	{
	public:
		enum Mode : int8_t
		{
			WRITE = 0,
			DELETE_THREAD = 1,
			LIST = 2,
			SHOW = 3,
			REPLY = 4,
			DELETE_REPLY = 5
		};

	protected:
		GuildBbsPacket(Mode mode) : OutPacket(OutPacket::Opcode::GUILD_BBS)
		{
			write_byte(mode);
		}
	};

	// Asks for a page of the board's threads, from 0
	class BbsListPacket : public GuildBbsPacket
	{
	public:
		BbsListPacket(int32_t page) : GuildBbsPacket(GuildBbsPacket::Mode::LIST)
		{
			write_int(page);
		}
	};

	// Asks for a thread; the notice is thread 0
	class BbsShowPacket : public GuildBbsPacket
	{
	public:
		BbsShowPacket(int32_t thread) : GuildBbsPacket(GuildBbsPacket::Mode::SHOW)
		{
			write_int(thread);
		}
	};

	// Writes a thread, the notice, or a thread's new version (0x8166f6)
	class BbsWritePacket : public GuildBbsPacket
	{
	public:
		// Pass the thread when editing one.
		BbsWritePacket(std::optional<int32_t> thread, bool notice, const std::string& title, const std::string& body,
					   int32_t icon) : GuildBbsPacket(GuildBbsPacket::Mode::WRITE)
		{
			write_byte(thread.has_value());

			if (thread)
				write_int(*thread);

			write_byte(notice);
			write_string(title);
			write_string(body);
			write_int(icon);
		}
	};

	// Replies to a thread
	class BbsReplyPacket : public GuildBbsPacket
	{
	public:
		BbsReplyPacket(int32_t thread, const std::string& text) : GuildBbsPacket(GuildBbsPacket::Mode::REPLY)
		{
			write_int(thread);
			write_string(text);
		}
	};

	// Deletes a thread
	class BbsDeleteThreadPacket : public GuildBbsPacket
	{
	public:
		BbsDeleteThreadPacket(int32_t thread) : GuildBbsPacket(GuildBbsPacket::Mode::DELETE_THREAD)
		{
			write_int(thread);
		}
	};

	// Deletes a reply to a thread
	class BbsDeleteReplyPacket : public GuildBbsPacket
	{
	public:
		BbsDeleteReplyPacket(int32_t thread, int32_t reply) : GuildBbsPacket(GuildBbsPacket::Mode::DELETE_REPLY)
		{
			write_int(thread);
			write_int(reply);
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

	// Opcode: MOB_DAMAGE_MOB_FRIENDLY(192)
	class MobDamageFriendlyPacket : public OutPacket
	{
	public:
		// Tells the server mob `attacker` ran into the friendly mob `victim`
		// (0x667f50)
		MobDamageFriendlyPacket(int32_t attacker, int32_t charid, int32_t victim) : OutPacket(
				OutPacket::Opcode::MOB_DAMAGE_MOB_FRIENDLY)
		{
			write_int(attacker);
			write_int(charid);
			write_int(victim);
		}
	};

	// Opcode: MOB_BOMB(193)
	class MobBombPacket : public OutPacket
	{
	public:
		// Tells the server a mob that bombs has the player in reach
		MobBombPacket(int32_t oid) : OutPacket(OutPacket::Opcode::MOB_BOMB)
		{
			write_int(oid);
		}
	};

	// Opcode: MOB_DAMAGE_MOB(194)
	class MobDamageMobPacket : public OutPacket
	{
	public:
		// Tells the server a mob's attack hurt a Hypnotized mob, which it
		// hit at `at` with attack `index` facing left or not (FUN_00670c63)
		MobDamageMobPacket(int32_t attacker, int32_t charid, int32_t victim, uint8_t index, int32_t damage, bool left,
						   Point<int16_t> at) : OutPacket(OutPacket::Opcode::MOB_DAMAGE_MOB)
		{
			write_int(attacker);
			write_int(charid);
			write_int(victim);
			write_byte(static_cast<int8_t>(index));
			write_int(damage);
			write_byte(left);
			write_short(at.x());
			write_short(at.y());
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

	// A pet standing at `position` reaches for a drop (v83 FUN_00705c7c).
	// The flags say whether its equipment lets it take others' drops, sweep
	// for drops and reach far. We send no checksum for an item; the server
	// doesn't read it.
	// Opcode: PET_LOOT(170)
	class PetLootPacket : public OutPacket
	{
	public:
		PetLootPacket(int64_t serial, Point<int16_t> position, int32_t oid, bool others, bool sweep, bool longrange)
			: OutPacket(OutPacket::Opcode::PET_LOOT)
		{
			write_long(serial);
			write_byte(0);
			write_time();
			write_point(position);
			write_int(oid);
			write_int(0);
			write_byte(others);
			write_byte(sweep);
			write_byte(longrange);
		}
	};

	// The first pet out, `serial`, gives the potion in USE slot `slot` from
	// beside its pouch (v83 FUN_00a0955c).
	// Opcode: PET_AUTO_POT(171)
	class PetAutoPotPacket : public OutPacket
	{
	public:
		PetAutoPotPacket(int64_t serial, int16_t slot, int32_t itemid) : OutPacket(OutPacket::Opcode::PET_AUTO_POT)
		{
			write_long(serial);
			write_byte(0);
			write_time();
			write_short(slot);
			write_int(itemid);
		}
	};

	// The player closed a pet's exception list (FUN_00706393): the ids it
	// is to leave, pets::IGNORED_MESOS for mesos.
	// Opcode: PET_EXCLUDE_ITEMS(172)
	class PetExcludeItemsPacket : public OutPacket
	{
	public:
		PetExcludeItemsPacket(int64_t serial, const std::vector<int32_t>& ids)
			: OutPacket(OutPacket::Opcode::PET_EXCLUDE_ITEMS)
		{
			write_long(serial);
			write_byte(static_cast<int8_t>(ids.size()));

			for (int32_t id : ids)
				write_int(id);
		}
	};

	// Hits a reactor (v83 FUN_007356c7). The kind is 0 for a swing and 1 for
	// a skill, the flags bit 0 for facing left and bit 1 for standing, and
	// the delay the ms until the hit lands, for others to see it then.
	// Opcode: DAMAGE_REACTOR(205)
	class DamageReactorPacket : public OutPacket
	{
	public:
		DamageReactorPacket(int32_t oid, int32_t kind, int32_t flags, int16_t delay, int32_t skillid)
			: OutPacket(OutPacket::Opcode::DAMAGE_REACTOR)
		{
			write_int(oid);
			write_int(kind);
			write_int(flags);
			write_short(delay);
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

	// Asks where the map's ship is, on entering a ship field (v83
	// FUN_0054db7a). The answer is CONTI_STATE.
	// Opcode: CONTI_STATE_REQUEST(220)
	class ContiStateRequestPacket : public OutPacket
	{
	public:
		ContiStateRequestPacket(int32_t mapid, uint8_t kind) : OutPacket(OutPacket::Opcode::CONTI_STATE_REQUEST)
		{
			write_int(mapid);
			write_byte(kind);
		}
	};

	// Tells the server Berserk turned on or off; it works out which itself
	// Opcode: CALC_DAMAGE_STAT(108)
	class CalcDamageStatPacket : public OutPacket
	{
	public:
		CalcDamageStatPacket() : OutPacket(OutPacket::Opcode::CALC_DAMAGE_STAT) {}
	};

	// Searches for party members (0xa3e264): the levels, the party size
	// wanted, and the job boxes.
	// Opcode: PARTY_SEARCH_START(222)
	class PartySearchStartPacket : public OutPacket
	{
	public:
		PartySearchStartPacket(int32_t min_level, int32_t max_level, int32_t members, int32_t jobs)
			: OutPacket(OutPacket::Opcode::PARTY_SEARCH_START)
		{
			write_int(min_level);
			write_int(max_level);
			write_int(members);
			write_int(jobs);
		}
	};

	// Pauses or stops a search (0xa3e2d7); v83 also sends it on field
	// entries that end one.
	// Opcode: PARTY_SEARCH_STOP(223)
	class PartySearchStopPacket : public OutPacket
	{
	public:
		PartySearchStopPacket() : OutPacket(OutPacket::Opcode::PARTY_SEARCH_STOP) {}
	};
}
