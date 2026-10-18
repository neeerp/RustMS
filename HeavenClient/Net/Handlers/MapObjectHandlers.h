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
	// Spawn a character on the stage
	// Opcode: SPAWN_CHAR(160)
	class SpawnCharHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Put up, change or take down the balloon over someone keeping a room
	// open (FUN_00938ba5)
	// Opcode: UPDATE_CHAR_BOX(165)
	class UpdateCharBoxHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Remove a character from the stage
	// Opcode: REMOVE_CHAR(161)
	class RemoveCharHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Spawn a pet on the stage
	// Opcode: SPAWN_PET(168)
	class SpawnPetHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Move a character
	// Opcode: CHAR_MOVED(185)
	class CharMovedHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show another character's emote
	// Opcode: FACIAL_EXPRESSION(193)
	class FacialExpressionHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show another character sitting on a portable chair, or getting up with 0
	// Opcode: SHOW_CHAIR(196)
	class ShowChairHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Let the player get up from a portable chair (mode 0), or seat them on
	// a map seat (mode 1, followed by its id)
	// Opcode: CANCEL_CHAIR(205)
	class CancelChairHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Update the look of a character
	// Opcode: UPDATE_CHARLOOK(197)
	class UpdateCharLookHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Display an effect on a character
	// Opcode: SHOW_FOREIGN_EFFECT(198)
	class ShowForeignEffectHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Another character started showing buffs
	// Opcode: GIVE_FOREIGN_BUFF(199)
	class GiveForeignBuffHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Another character stopped showing buffs
	// Opcode: CANCEL_FOREIGN_BUFF(200)
	class CancelForeignBuffHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Spawn a mob on the stage
	// Opcode: SPAWN_MOB(236)
	class SpawnMobHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Remove a map from the stage, either by killing it or making it invisible.
	// Opcode: KILL_MOB(237)
	class KillMobHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Spawn a mob on the stage and take control of it
	// Opcode: SPAWN_MOB_C(238)
	class SpawnMobControllerHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// The server got the controller's move of a mob (FUN_0066c23b).
	// Opcode: MOVE_MOB_RESPONSE(240)
	class MoveMobResponseHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Update mob state and position with the client
	// Opcode: MOB_MOVED(239)
	class MobMovedHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show a number on a mob that no player's attack made: damage, or a
	// heal as a negative one (FUN_0066c6c2).
	// Opcode: DAMAGE_MOB(246)
	class DamageMobHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Updates a mob's hp with the client
	// Opcode: SHOW_MOB_HP(250)
	class ShowMobHpHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Put temporary statuses on a mob (FUN_0066c301).
	// Opcode: MOB_STAT_SET(242)
	class MobStatSetHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Take temporary statuses off a mob (FUN_0066c424).
	// Opcode: MOB_STAT_RESET(243)
	class MobStatResetHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Show another user's Monster Magnet catching a mob, or failing to
	// (0x66d6b9).
	// Opcode: SHOW_MAGNET(251)
	class ShowMagnetHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Spawn an NPC on the current stage
	// Opcode: SPAWN_NPC(257)
	class SpawnNpcHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Remove an NPC from the stage (FUN_006d9a25)
	// Opcode: REMOVE_NPC(258)
	class RemoveNpcHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Take control of an NPC, spawning it if need be, or give it up
	// (FUN_006d9a83). The controller picks the NPC's idle actions.
	// Opcode: SPAWN_NPC_C(259)
	class SpawnNpcControllerHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Play the idle action the NPC's controller chose (FUN_006d2e07)
	// Opcode: NPC_ACTION(260)
	class NpcActionHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Drop an item on the stage
	// Opcode: DROP_LOOT(268)
	class DropLootHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Remove an item from the stage
	// Opcode: REMOVE_LOOT(269)
	class RemoveLootHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Cover an area with a skill's clouds
	// Opcode: SPAWN_MIST(273)
	class SpawnMistHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Fade a mist out
	// Opcode: REMOVE_MIST(274)
	class RemoveMistHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A Mystic Door appears in the field
	// Opcode: SPAWN_DOOR(275)
	class SpawnDoorHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A Mystic Door closes
	// Opcode: REMOVE_DOOR(276)
	class RemoveDoorHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Where the player's own Mystic Door leads, or that it closed
	// Opcode: SPAWN_PORTAL(67)
	class SpawnPortalHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A user's summon appears
	// Opcode: SPAWN_SUMMON(175)
	class SpawnSummonHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A summon goes; the player reads why theirs did
	// Opcode: REMOVE_SUMMON(176)
	class RemoveSummonHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Another user's summon moves
	// Opcode: MOVE_SUMMON(177)
	class MoveSummonHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Another user's summon attacks
	// Opcode: SUMMON_ATTACK(178)
	class SummonAttackHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Another user's Beholder heals or buffs them
	// Opcode: SUMMON_SKILL(179)
	class SummonSkillHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// A mob hurts another user's Puppet
	// Opcode: DAMAGE_SUMMON(180)
	class DamageSummonHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Change state of reactor
	// Opcode: HIT_REACTOR(277)
	class HitReactorHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Parse a ReactorSpawn and send it to the Stage spawn queue
	// Opcode: SPAWN_REACTOR(279)
	class SpawnReactorHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};

	// Remove a reactor from the stage
	// Opcode: REMOVE_REACTOR(280)
	class RemoveReactorHandler : public PacketHandler
	{
		void handle(InPacket& recv) const override;
	};
}