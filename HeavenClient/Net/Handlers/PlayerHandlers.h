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

#include "Helpers/BuffParser.h"

#include "../../Character/Buff.h"
#include "../../Character/MapleStat.h"

namespace ms
{
	// Handles the changing of channels for players
	// Opcode: CHANGE_CHANNEL(16)
	class ChangeChannelHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Notifies the client of changes in character stats
	// Opcode: CHANGE_STATS(31)
	class ChangeStatsHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;

	private:
		bool handle_stat(MapleStat::Id stat, InPacket& recv) const;
		bool need_statsinfo_update(MapleStat::Id stat) const;
		bool need_skillbook_update(MapleStat::Id stat) const;
	};

	// Base class for packets which need to parse buff stats
	class BuffHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;

	protected:
		virtual void handle_buff(InPacket& recv, Buffstat::Id stat) const = 0;

		// Energy Charge's and riding's bits, after the other stats (reports 194, 201).
		virtual void handle_two_state(InPacket& recv, const BuffParser::Flags& flags) const = 0;
	};

	// Notifies the client that a buff was applied to the player
	// Opcode: GIVE_BUFF(32)
	class ApplyBuffHandler : public BuffHandler
	{
	protected:
		void handle_buff(InPacket& recv, Buffstat::Id stat) const override;
		void handle_two_state(InPacket& recv, const BuffParser::Flags& flags) const override;
	};

	// Notifies the client that a buff was canceled
	// Opcode: CANCEL_BUFF(33)
	class CancelBuffHandler : public BuffHandler
	{
	protected:
		void handle_buff(InPacket& recv, Buffstat::Id stat) const override;
		void handle_two_state(InPacket& recv, const BuffParser::Flags& flags) const override;
	};

	// Force a stat recalculation
	// Opcode: RECALCULATE_STATS(35)
	class RecalculateStatsHandler : public PacketHandler
	{
		void handle(InPacket&) const override;
	};

	// A character's mount's level, EXP and tiredness, and whether it just reached the level
	// Opcode: SET_TAMING_MOB_INFO(48)
	class SetTamingMobInfoHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Updates the player's skills with the client
	// Opcode: UPDATE_SKILL(36)
	class UpdateSkillHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handler for a packet which answers the player's skill use (v83
	// FUN_00a209b2): it lets the player send the next request.
	class SkillUseResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Handler for a packet which tells how someone's skill or mastery book
	// went (v83 FUN_00a1e5af).
	class SkillLearnItemResultHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Parses skill macros
	// Opcode: SKILL_MACROS(124)
	class SkillMacrosHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Sets the quick slot keys, or the defaults (FUN_0072b82a)
	// Opcode: QUICKSLOT_INIT(159)
	class QuickSlotInitHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Notifies the client that a skill is on cool-down
	// Opcode: ADD_COOLDOWN(234)
	class AddCooldownHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Parses key mappings and sends them to the keyboard
	// Opcode: KEYMAP(335)
	class KeymapHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The potion the first pet gives from beside its HP pouch, 0 for none
	// (v83 FUN_0058de2d)
	// Opcode: AUTO_HP_POT(336)
	class AutoHpPotHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The same for the MP pouch (FUN_0058de53)
	// Opcode: AUTO_MP_POT(337)
	class AutoMpPotHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Checks the HP potion as if HP were lower by the amount sent, for HP
	// the server takes without a hit (FUN_0095b985)
	// Opcode: CHECK_AUTO_HP(212)
	class CheckAutoHpHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Puts a picked-up card in the Monster Book, or says its page is full
	// Opcode: MONSTER_BOOK_SET_CARD(83)
	class MonsterBookSetCardHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Shows a card on the Monster Book's cover
	// Opcode: MONSTER_BOOK_SET_COVER(84)
	class MonsterBookSetCoverHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};
}
