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
#include "MapObjectHandlers.h"

#include "Helpers/BuffParser.h"
#include "Helpers/LoginParser.h"
#include "Helpers/MobStatParser.h"
#include "Helpers/MovementParser.h"

#include "../../Gameplay/Stage.h"

namespace ms
{
	void SpawnCharHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();

		// We don't need to spawn the player twice
		if (Stage::get().is_player(cid))
			return;

		uint8_t level = recv.read_byte();
		std::string name = recv.read_string();

		recv.read_string();	// guildname
		recv.read_short();	// guildlogobg
		recv.read_byte();	// guildlogobgcolor
		recv.read_short();	// guildlogo
		recv.read_byte();	// guildlogocolor

		BuffParser::Flags flags = BuffParser::read_flags(recv);
		auto buffs = BuffParser::parse_shown(recv, flags);

		int16_t job = recv.read_short();
		LookEntry look = LoginParser::parse_look(recv);

		recv.read_int(); // count of 5110000 
		recv.read_int(); // 'itemeffect'
		int32_t chair = recv.read_int();

		Point<int16_t> position = recv.read_point();
		int8_t stance = recv.read_byte();

		recv.skip(3);

		for (size_t i = 0; i < 3; i++)
		{
			int8_t available = recv.read_byte();

			if (available == 1)
			{
				recv.read_byte();	// 'byte2'
				recv.read_int();	// petid
				recv.read_string();	// name
				recv.read_int();	// unique id
				recv.read_int();
				recv.read_point();	// pos
				recv.read_byte();	// stance
				recv.read_int();	// fhid
			}
			else
			{
				break;
			}
		}

		recv.read_int(); // mountlevel
		recv.read_int(); // mountexp
		recv.read_int(); // mounttiredness

		// TODO: Shop stuff
		recv.read_byte();
		// TODO: Shop stuff end

		bool chalkboard = recv.read_bool();
		std::string chalktext = chalkboard ? recv.read_string() : "";

		recv.skip(3);
		recv.read_byte(); // team

		Stage::get().get_chars().spawn(
			{ cid, look, level, job, name, stance, position, chair, std::move(buffs) }
		);
	}

	void GiveForeignBuffHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		BuffParser::Flags flags = BuffParser::read_flags(recv);
		auto buffs = BuffParser::parse_shown(recv, flags);

		if (Optional<OtherChar> otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->show_buffs(buffs);
	}

	void CancelForeignBuffHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		BuffParser::Flags flags = BuffParser::read_flags(recv);

		if (Optional<OtherChar> otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->hide_buffs(BuffParser::stats(flags));
	}

	void RemoveCharHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();

		Stage::get().get_chars().remove(cid);
	}

	void SpawnPetHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		Optional<Char> character = Stage::get().get_character(cid);

		if (!character)
			return;

		uint8_t petindex = recv.read_byte();
		int8_t mode = recv.read_byte();

		if (mode == 1)
		{
			recv.skip(1);

			int32_t itemid = recv.read_int();
			std::string name = recv.read_string();
			int32_t uniqueid = recv.read_int();

			recv.skip(4);

			Point<int16_t> pos = recv.read_point();
			uint8_t stance = recv.read_byte();
			int32_t fhid = recv.read_int();

			character->add_pet(petindex, itemid, name, uniqueid, pos, stance, fhid);
		}
		else if (mode == 0)
		{
			bool hunger = recv.read_bool();

			character->remove_pet(petindex, hunger);
		}
	}

	void CharMovedHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		Point<int16_t> start = recv.read_point();
		std::vector<Movement> movements = MovementParser::parse_movements(recv, start);

		Stage::get().get_chars().send_movement(cid, movements);
	}

	void FacialExpressionHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t emote = recv.read_int();

		// The inverse of the FACE key action to emote mapping in Stage::send_key.
		if (Optional<OtherChar> otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->set_expression(emote + 99);
	}

	void ShowChairHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t itemid = recv.read_int();

		if (Optional<OtherChar> otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->set_chair(itemid);
	}

	void CancelChairHandler::handle(InPacket& recv) const
	{
		if (recv.read_byte() == 0)
			Stage::get().get_player().stand_up();
	}

	void UpdateCharLookHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		recv.read_byte();
		LookEntry look = LoginParser::parse_look(recv);

		Stage::get().get_chars().update_look(cid, look);
	}

	void ShowForeignEffectHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int8_t effect = recv.read_byte();

		// v83 special effects: 0 level up, 8 job change, 9 quest complete.
		if (effect == 0 && !recv.available())
		{
			Stage::get().show_character_effect(cid, CharEffect::LEVELUP);
		}
		else if (effect == 8 && !recv.available())
		{
			Stage::get().show_character_effect(cid, CharEffect::JOBCHANGE);
		}
		else if (effect == 9 && !recv.available())
		{
			Stage::get().show_character_effect(cid, CharEffect::QUEST_CLEAR);
		}
		else if (effect == 10) // recovery
		{
			recv.read_byte(); // 'amount'
		}
		else if (effect == 13) // card effect
		{
			Stage::get().show_character_effect(cid, CharEffect::MONSTER_CARD);
		}
		else if (effect == 2) // a party member's skill reached them
		{
			int32_t skillid = recv.read_int();
			recv.read_byte(); // level

			Stage::get().get_combat().show_affected(cid, skillid);
		}
		else if (recv.available()) // skill
		{
			int32_t skillid = recv.read_int();
			recv.read_byte(); // 'direction'
			// 9 more bytes after this

			Stage::get().get_combat().show_buff(cid, skillid, effect);
		}
		else
		{
			// TODO: Blank
		}
	}

	void SpawnMobHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		recv.read_byte(); // 5 if controller == null
		int32_t id = recv.read_int();

		BuffParser::Flags flags = BuffParser::read_flags(recv);
		std::vector<MobStatParser::Status> statuses = MobStatParser::parse(recv, flags);

		Point<int16_t> position = recv.read_point();
		int8_t stance = recv.read_byte();

		// The foothold the mob stands on (0 if airborne), then the one it
		// spawned on.
		uint16_t fh = recv.read_short();
		uint16_t homefh = recv.read_short();
		int8_t effect = recv.read_byte();

		if (effect > 0)
		{
			recv.read_byte();
			recv.read_short();

			if (effect == 15)
				recv.read_byte();
		}

		int8_t team = recv.read_byte();

		recv.skip(4);

		Stage::get().get_mobs().spawn(
			{ oid, id, 0, stance, fh, homefh, effect == -2, team, position, std::move(statuses) }
		);
	}

	void KillMobHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int8_t animation = recv.read_byte();

		Stage::get().get_mobs().remove(oid, animation);
	}

	void SpawnMobControllerHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();
		int32_t oid = recv.read_int();

		if (mode == 0)
		{
			Stage::get().get_mobs().set_control(oid, false);
		}
		else
		{
			if (recv.available())
			{
				recv.skip(1);

				int32_t id = recv.read_int();

				BuffParser::Flags flags = BuffParser::read_flags(recv);
				std::vector<MobStatParser::Status> statuses = MobStatParser::parse(recv, flags);

				Point<int16_t> position = recv.read_point();
				int8_t stance = recv.read_byte();

				uint16_t fh = recv.read_short();
				uint16_t homefh = recv.read_short();
				int8_t effect = recv.read_byte();

				if (effect > 0)
				{
					recv.read_byte();
					recv.read_short();

					if (effect == 15)
						recv.read_byte();
				}

				int8_t team = recv.read_byte();

				recv.skip(4);

				Stage::get().get_mobs().spawn(
					{ oid, id, mode, stance, fh, homefh, effect == -2, team, position, std::move(statuses) }
				);
			}
			else
			{
				// TODO: Remove monster invisibility, not used (maybe in an event script?), Check this!
			}
		}
	}

	void MobMovedHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();

		recv.read_byte();
		recv.read_byte(); // useskill
		recv.read_byte(); // skill
		recv.read_byte(); // skill 1
		recv.read_byte(); // skill 2
		recv.read_byte(); // skill 3
		recv.read_byte(); // skill 4

		Point<int16_t> position = recv.read_point();
		std::vector<Movement> movements = MovementParser::parse_movements(recv, position);

		Stage::get().get_mobs().send_movement(oid, position, std::move(movements));
	}

	void MoveMobResponseHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int16_t moveid = recv.read_short();
		bool nextattack = recv.read_bool();

		// Its MP and a skill to cast next: mob skills aren't modeled yet.
		recv.read_short();
		recv.read_byte();
		recv.read_byte();

		Stage::get().get_mobs().acknowledge(oid, moveid, nextattack);
	}

	void ShowMobHpHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int8_t hppercent = recv.read_byte();
		uint16_t playerlevel = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::LEVEL);

		Stage::get().get_mobs().send_mobhp(oid, hppercent, playerlevel);
	}

	void MobStatSetHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		BuffParser::Flags flags = BuffParser::read_flags(recv);
		std::vector<MobStatParser::Status> statuses = MobStatParser::parse(recv, flags);

		// The delay before its damage stats change, the stat they're
		// calculated from, and with Stun, Freeze or Doom the mob's next
		// action (FUN_00671b71).
		recv.read_short();
		recv.read_byte();

		if (MobStatParser::stops(flags))
			recv.read_byte();

		Stage::get().get_mobs().set_statuses(oid, statuses);
	}

	void MobStatResetHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		BuffParser::Flags flags = BuffParser::read_flags(recv);

		MobStatParser::skip_reset(recv, flags);

		Stage::get().get_mobs().reset_statuses(oid, flags);
	}

	void ShowMagnetHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		bool success = recv.read_bool();

		Stage::get().get_combat().show_catch(oid, success);
	}

	void SpawnNpcHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int32_t id = recv.read_int();
		Point<int16_t> position = recv.read_point();
		bool flip = recv.read_bool();
		uint16_t fh = recv.read_short();

		recv.read_short(); // 'rx0'
		recv.read_short(); // 'rx1'

		Stage::get().get_npcs().spawn(
			{ oid, id, position, flip, fh }
		);
	}

	void RemoveNpcHandler::handle(InPacket& recv) const
	{
		Stage::get().get_npcs().remove(recv.read_int());
	}

	void SpawnNpcControllerHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();
		int32_t oid = recv.read_int();

		if (mode == 0)
		{
			Stage::get().get_npcs().control(oid, false);
		}
		else
		{
			int32_t id = recv.read_int();
			Point<int16_t> position = recv.read_point();
			bool flip = recv.read_bool();
			uint16_t fh = recv.read_short();

			recv.read_short();	// 'rx0'
			recv.read_short();	// 'rx1'
			recv.read_bool();	// 'minimap'

			// Spawned already, the NPC only changes hands.
			Stage::get().get_npcs().spawn(
				{ oid, id, position, flip, fh }
			);
			Stage::get().get_npcs().control(oid, true);
		}
	}

	void NpcActionHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int8_t action = recv.read_byte();
		int8_t line = recv.read_byte();

		// A path follows for an NPC that walks; none does in v83.
		Stage::get().get_npcs().act(oid, action, line);
	}

	void DropLootHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();
		int32_t oid = recv.read_int();
		bool meso = recv.read_bool();
		int32_t itemid = recv.read_int();
		int32_t owner = recv.read_int();
		int8_t pickuptype = recv.read_byte();
		Point<int16_t> dropto = recv.read_point();

		recv.skip(4);

		Point<int16_t> dropfrom;
		int16_t delay = 0;

		if (mode != 2)
		{
			dropfrom = recv.read_point();
			delay = recv.read_short();
		}
		else
		{
			dropfrom = dropto;
		}

		if (!meso)
			recv.skip(8);

		bool playerdrop = !recv.read_bool();

		Stage::get().get_drops().spawn(
			{ oid, itemid, meso, owner, dropfrom, dropto, pickuptype, mode, delay, playerdrop }
		);
	}

	void RemoveLootHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();
		int32_t oid = recv.read_int();

		Optional<PhysicsObject> looter;

		if (mode > 1)
		{
			int32_t cid = recv.read_int();

			// A pet's loot flies to its owner.
			if (recv.length() > 0)
				recv.read_byte(); // pet

			if (auto character = Stage::get().get_character(cid))
				looter = character->get_phobj();

			if (Stage::get().is_player(cid))
				Sound(Sound::Name::PICKUP).play();
		}

		Stage::get().get_drops().remove(oid, mode, looter.get());
	}

	void HitReactorHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int8_t state = recv.read_byte();
		Point<int16_t> point = recv.read_point();
		int8_t stance = recv.read_byte(); // TODO: When is this different than state?
		recv.skip(2); // TODO: Unused
		recv.skip(1); // "frame" delay but this is in the WZ file?

		Stage::get().get_reactors().trigger(oid, state);
	}

	void SpawnReactorHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int32_t rid = recv.read_int();
		int8_t state = recv.read_byte();
		Point<int16_t> point = recv.read_point();

		// TODO: Unused, Check this!
		// uint16_t fhid = recv.read_short();
		// recv.read_byte()

		Stage::get().get_reactors().spawn(
			{ oid, rid, state, point }
		);
	}

	void SpawnMistHandler::handle(InPacket& recv) const
	{
		// v83 FUN_00431a63. The delay (in 100 ms) only starts the mist's own
		// clock; the clouds show at once. The last int is unused here.
		int32_t oid = recv.read_int();
		int32_t type = recv.read_int();
		recv.skip_int();
		int32_t skillid = recv.read_int();
		uint8_t level = recv.read_byte();
		recv.skip_short();
		int32_t left = recv.read_int();
		int32_t top = recv.read_int();
		int32_t right = recv.read_int();
		int32_t bottom = recv.read_int();

		Stage::get().get_mists().spawn(oid, type, skillid, level, left, top, right, bottom);
	}

	void RemoveMistHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();

		Stage::get().get_mists().remove(oid);
	}

	void RemoveReactorHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int8_t state = recv.read_byte();
		Point<int16_t> point = recv.read_point();

		Stage::get().get_reactors().remove(oid, state, point);
	}
}