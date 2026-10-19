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
#include "Helpers/GuildParser.h"
#include "Helpers/LoginParser.h"
#include "Helpers/MobStatParser.h"
#include "Helpers/MovementParser.h"
#include "Helpers/PetParser.h"
#include "MiniRoom.h"

#include "../../Data/CurseData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../Util/Chalkboard.h"
#include "../Packets/GameplayPackets.h"

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

		std::string guild = recv.read_string();
		GuildEmblem emblem = GuildParser::parse_emblem(recv);

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

		std::vector<PetEntry> pets = PetParser::parse_pets(recv);

		recv.read_int(); // mountlevel
		recv.read_int(); // mountexp
		recv.read_int(); // mounttiredness

		std::optional<MiniRoomBox> room = mini_room::read_box(recv);

		std::optional<std::string> chalkboard;

		if (recv.read_bool())
			chalkboard = Chalkboard::remote_text(recv.read_string(), CurseData::filter);

		recv.skip(3);
		recv.read_byte(); // team

		Stage::get().get_chars().spawn(
			{ cid, look, level, job, name, stance, position, chair, std::move(buffs), guild, emblem, room, std::move(pets),
			  chalkboard }
		);
	}

	void ChalkboardHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		std::optional<std::string> text;

		if (recv.read_bool())
			text = recv.read_string();

		// Only the player's own board can be closed, and only someone else's
		// note is filtered and cut.
		if (Stage::get().is_player(cid))
			Stage::get().get_player().set_chalkboard(text ? Chalkboard::trim(*text) : text, true);
		else if (Optional<OtherChar> other = Stage::get().get_chars().get_char(cid))
			other->set_chalkboard(text ? Chalkboard::remote_text(*text, CurseData::filter) : text, false);
	}

	void UpdateCharBoxHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		std::optional<MiniRoomBox> room = mini_room::read_box(recv);

		// The owner sees their own balloon too.
		if (Stage::get().is_player(cid))
			Stage::get().get_player().set_mini_room(room);
		else if (Optional<OtherChar> other = Stage::get().get_chars().get_char(cid))
			other->set_mini_room(room);
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
		Stage::get().get_summons().remove_owner(cid);
	}

	void SpawnPetHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		Optional<Char> character = Stage::get().get_character(cid);

		if (!character)
			return;

		// Index 3 asks the owner which pet the Water of Life revives.
		uint8_t index = recv.read_byte();

		if (index > 2)
			return;

		bool local = Stage::get().is_player(cid);

		if (recv.read_bool())
		{
			// A nonzero byte replaces the pet at the index.
			bool replace = recv.read_bool();

			character->add_pet(index, replace, PetParser::parse_pet(recv), local);
		}
		else
		{
			character->remove_pet(index);

			if (!local)
				return;

			// Why the pet went back, which only the owner reads (95a947).
			const char *reason = nullptr;

			switch (recv.read_byte())
			{
				case 1:
					reason = "The pet went back home because it's hungry.";
					break;
				case 2:
					reason = "The pet's magical time has run out, and so it has turned back into a doll.";
					break;
				case 3:
					reason = "You cannot use pet here.";
					break;
			}

			if (reason)
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(reason, UIChatBar::LineType::SKILL);
		}
	}

	void MovePetHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		uint8_t index = recv.read_byte();
		Point<int16_t> start = recv.read_point();
		std::vector<Movement> movements = MovementParser::parse_movements(recv, start);

		// The player's pets move themselves.
		if (Optional<OtherChar> otherchar = Stage::get().get_chars().get_char(cid))
			otherchar->send_pet_movement(index, movements);
	}

	void PetChatHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		uint8_t index = recv.read_byte();
		uint8_t type = recv.read_byte();
		int8_t action = recv.read_byte();
		std::string text = recv.read_string();
		bool styled = recv.read_bool();

		if (Optional<Char> character = Stage::get().get_character(cid))
			if (Optional<PetLook> pet = character->get_pet(index))
				pet->act({ type, action, text }, styled);
	}

	void PetNameChangeHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		uint8_t index = recv.read_byte();
		std::string name = recv.read_string();

		// The name tag's style (`info/nameTag`) when set; HeavenClient
		// draws one plain tag for every pet.
		recv.read_bool();

		if (Optional<Char> character = Stage::get().get_character(cid))
			if (Optional<PetLook> pet = character->get_pet(index))
				pet->set_name(name);
	}

	void CashPetFoodResultHandler::handle(InPacket& recv) const
	{
		switch (recv.read_byte())
		{
			case 0:
				if (Optional<PetLook> pet = Stage::get().get_player().get_pet(recv.read_byte()))
					pet->play_eat_sound();

				break;
			case 1:
				// StringPool 0xed1.
				UI::get().emplace<UIOk>("Your pet cannot consume this food. Please check again.", [](bool)
				{});
				break;
		}
	}

	void PetCommandHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();

		// Another's pet isn't in this player's Cash tab, so it has no level
		// and answers nothing (FUN_00703b69).
		if (!Stage::get().is_player(cid))
			return;

		uint8_t index = recv.read_byte();

		// 0 answers a command, 1 food.
		uint8_t mode = recv.read_byte();

		if (mode > 1)
			return;

		uint8_t command = mode == 0 ? recv.read_byte() : 0;
		bool obeyed = recv.read_bool();
		bool styled = recv.read_bool();

		Player& player = Stage::get().get_player();
		Optional<PetLook> pet = player.get_pet(index);

		if (!pet)
			return;

		int16_t level = player.get_inventory().get_pet_level(pet->get_serial());

		std::optional<PetLook::Said> said =
			mode == 0 ? pet->answer(command, obeyed, styled, level) : pet->eat(obeyed, styled, level);

		// The others see what it did when the server passes this on.
		if (said)
			PetChatPacket(pet->get_serial(), said->type, said->action, said->text).dispatch();
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

	namespace
	{
		// v83 (FUN_009377d9) plays these effects' sounds for anyone on the map, at full volume.
		void show_effect_with_sound(int32_t cid, CharEffect::Id effect, Sound::Name sound)
		{
			if (!Stage::get().get_character(cid))
				return;

			Stage::get().show_character_effect(cid, effect);
			Sound(sound).play();
		}
	}

	void ShowForeignEffectHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int8_t effect = recv.read_byte();

		// v83 special effects: 0 level up, 8 job change, 9 quest complete.
		if (effect == 0 && !recv.available())
		{
			show_effect_with_sound(cid, CharEffect::LEVELUP, Sound::Name::LEVELUP);
		}
		else if (effect == 8 && !recv.available())
		{
			show_effect_with_sound(cid, CharEffect::JOBCHANGE, Sound::Name::JOBCHANGED);
		}
		else if (effect == 9 && !recv.available())
		{
			show_effect_with_sound(cid, CharEffect::QUEST_CLEAR, Sound::Name::QUESTCLEAR);
		}
		else if (effect == 10) // recovery
		{
			recv.read_byte(); // 'amount'
		}
		else if (effect == 13) // card effect
		{
			show_effect_with_sound(cid, CharEffect::MONSTER_CARD, Sound::Name::MCARDGET);
		}
		else if (effect == 4) // a pet effect
		{
			uint8_t kind = recv.read_byte();
			uint8_t index = recv.read_byte();

			if (Optional<Char> character = Stage::get().get_character(cid))
				if (Optional<PetLook> pet = character->get_pet(index))
					pet->show_effect(kind);
		}
		else if (effect == 5) // a skill's special animation, as Dragon Blood's drink
		{
			Stage::get().get_combat().show_special(cid, recv.read_int());
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
		// How it appears (FUN_00662884): -2 fading in, -1 just there, -3
		// when the mob an int names dies, or a summon effect to play an
		// int's ms later.
		int8_t effect = recv.read_byte();
		int32_t option = effect >= 0 || effect == -3 ? recv.read_int() : 0;
		int8_t team = recv.read_byte();

		recv.skip(4);

		Stage::get().get_mobs().spawn(
			{ oid, id, 0, stance, fh, homefh, effect == -2, team, position, std::move(statuses), effect, option }
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
				// How it appears (FUN_00662884): -2 fading in, -1 just there,
				// -3 when the mob an int names dies, or a summon effect to
				// play an int's ms later.
				int8_t effect = recv.read_byte();
				int32_t option = effect >= 0 || effect == -3 ? recv.read_int() : 0;
				int8_t team = recv.read_byte();

				recv.skip(4);

				Stage::get().get_mobs().spawn(
					{ oid, id, mode, stance, fh, homefh, effect == -2, team, position, std::move(statuses),
					  effect, option }
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
		// FUN_0066be61: whether it may skip landing, whether it may attack
		// next, what it did (action << 1 | left, 0xFF for nothing), and the
		// attack's target or the skill's id and level.
		int32_t oid = recv.read_int();

		recv.read_bool(); // not forced to land
		recv.read_bool(); // may attack next
		auto actionbyte = static_cast<uint8_t>(recv.read_byte());
		int32_t skilldata = recv.read_int();

		int8_t action = actionbyte == 0xFF ? -1 : static_cast<int8_t>(actionbyte >> 1);
		bool left = (actionbyte & 1) != 0;

		Point<int16_t> position = recv.read_point();
		std::vector<Movement> movements = MovementParser::parse_movements(recv, position);

		Stage::get().get_mobs().send_movement(oid, position, std::move(movements), action, left, skilldata);
	}

	void MoveMobResponseHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		int16_t moveid = recv.read_short();
		bool nextattack = recv.read_bool();

		// Its MP, and a skill it may cast next, at what level.
		int16_t mp = recv.read_short();
		uint8_t skill = recv.read_byte();
		uint8_t skilllevel = recv.read_byte();

		Stage::get().get_mobs().acknowledge(oid, moveid, nextattack, mp, skill, skilllevel);
	}

	void DamageMobHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();
		recv.read_byte();
		int32_t damage = recv.read_int();

		// An HP indicator may follow; SHOW_MOB_HP shows HP here.
		Stage::get().get_mobs().show_damage(oid, damage);
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

		// Mode 4 sets the drop off for Meso Explosion, in `delay` ms
		// (FUN_00506590).
		if (mode == 4)
		{
			Stage::get().get_drops().explode(oid, recv.read_short());
			return;
		}

		const PhysicsObject *looter = nullptr;

		if (mode > 1)
		{
			int32_t cid = recv.read_int();

			if (auto character = Stage::get().get_character(cid))
			{
				// Mode 5 names the pet in its owner's list that took it, and
				// it flies to that pet, or vanishes if the pet isn't out
				// (FUN_004416db).
				if (mode == 5)
				{
					if (auto pet = character->get_pet(recv.read_byte()))
						looter = &pet->get_phobj();
				}
				else
				{
					looter = &character->get_phobj();
				}
			}

			if (Stage::get().is_player(cid))
				Sound(Sound::Name::PICKUP).play();
		}

		Stage::get().get_drops().remove(oid, mode, looter);
	}

	void HitReactorHandler::handle(InPacket& recv) const
	{
		// v83 FUN_0073502d. The reactor moved on to `state`; the hit, the
		// state before's event `event` (or its own hit for -1), plays after
		// `delay` ms, and it takes no swing for `cooldown` hundred ms.
		int32_t oid = recv.read_int();
		int8_t state = recv.read_byte();
		recv.read_point();
		uint16_t delay = recv.read_short();
		int8_t event = recv.read_byte();
		int8_t cooldown = recv.read_byte();

		Stage::get().get_reactors().hit(oid, state, delay, event, cooldown * 100);
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
		// clock; the clouds show at once. The last int is its element.
		int32_t oid = recv.read_int();
		int32_t type = recv.read_int();
		int32_t owner = recv.read_int();
		int32_t skillid = recv.read_int();
		uint8_t level = recv.read_byte();
		int16_t delay = recv.read_short();
		int32_t left = recv.read_int();
		int32_t top = recv.read_int();
		int32_t right = recv.read_int();
		int32_t bottom = recv.read_int();
		int32_t element = recv.read_int();

		Stage::get().get_mists().spawn(oid, type, owner, skillid, level, delay, left, top, right, bottom, element);
	}

	void RemoveMistHandler::handle(InPacket& recv) const
	{
		int32_t oid = recv.read_int();

		Stage::get().get_mists().remove(oid);
	}

	void SpawnDoorHandler::handle(InPacket& recv) const
	{
		// v83 FUN_007bd6c6: one just cast (0) opens, one cast before stands.
		bool just_cast = recv.read_byte() == 0;
		int32_t owner = recv.read_int();
		Point<int16_t> position = recv.read_point();

		Stage::get().get_doors().spawn(owner, position, just_cast);
	}

	void RemoveDoorHandler::handle(InPacket& recv) const
	{
		// v83 FUN_007be064 fades the door out unless the byte is set.
		bool fade = recv.read_byte() == 0;
		int32_t owner = recv.read_int();

		Stage::get().get_doors().remove(owner, fade);
	}

	void SpawnPortalHandler::handle(InPacket& recv) const
	{
		// v83 FUN_00a226a6. The town doors are drawn from this each frame.
		MysticDoor door;
		door.town = recv.read_int();
		door.field = recv.read_int();

		if (door.town != MysticDoor::NONE && door.field != MysticDoor::NONE)
			door.position = recv.read_point();

		Stage::get().get_player().get_door() = door;
	}

	void SpawnSummonHandler::handle(InPacket& recv) const
	{
		// v83 FUN_0095adec (the player's) and FUN_00938f61 (another
		// user's). The foothold is found again here.
		int32_t owner = recv.read_int();
		int32_t oid = recv.read_int();
		int32_t skillid = recv.read_int();
		recv.skip_byte();
		uint8_t level = recv.read_byte();
		Point<int16_t> position = recv.read_point();
		uint8_t action = recv.read_byte();
		recv.skip_short();
		uint8_t moves = recv.read_byte();
		uint8_t assist = recv.read_byte();
		bool appear = recv.length() > 0 ? recv.read_byte() == 0 : true;

		bool local = Stage::get().is_player(owner);
		Stage::get().get_summons().spawn(owner, oid, skillid, level, position, action, moves, assist, appear, local);

		// The mobs this client moves that chase someone turn on the player's
		// Puppet (FUN_00679847).
		Summon *summon = Stage::get().get_summons().get(oid);

		if (local && summon && summon_rules::is_puppet(skillid))
			Stage::get().get_mobs().aggro_puppet(oid, summon->get_position());
	}

	void RemoveSummonHandler::handle(InPacket& recv) const
	{
		// v83 FUN_007a64eb.
		int32_t owner = recv.read_int();
		int32_t oid = recv.read_int();
		uint8_t reason = recv.read_byte();

		int32_t skillid = Stage::get().get_summons().remove(oid, reason);
		Stage::get().get_mobs().release_puppet(oid);

		if (skillid == 0 || !Stage::get().is_player(owner) || (reason != 0 && reason != 3))
			return;

		const std::string &name = SkillData::get(skillid).get_name();
		std::string message = reason == 0 ? name + "'s time has run out, and will disappear."
										  : name + " is disappearing.";

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(message, UIChatBar::LineType::SKILL);
	}

	void MoveSummonHandler::handle(InPacket& recv) const
	{
		recv.skip_int();
		int32_t oid = recv.read_int();
		Point<int16_t> start = recv.read_point();
		std::vector<Movement> movements = MovementParser::parse_movements(recv, start);

		Stage::get().get_summons().move(oid, movements);
	}

	void SummonAttackHandler::handle(InPacket& recv) const
	{
		// v83 FUN_007a6882: one hit on each mob.
		recv.skip_int();
		int32_t oid = recv.read_int();
		recv.skip_byte();
		uint8_t action = recv.read_byte();
		uint8_t count = recv.read_byte();
		std::vector<std::pair<int32_t, int32_t>> hits;

		for (uint8_t i = 0; i < count; i++)
		{
			int32_t mob = recv.read_int();

			if (mob == 0)
				continue;

			recv.skip_byte();
			hits.emplace_back(mob, recv.read_int());
		}

		Stage::get().get_combat().show_summon_attack(oid, action, hits);
	}

	void SummonSkillHandler::handle(InPacket& recv) const
	{
		recv.skip_int();
		int32_t oid = recv.read_int();
		uint8_t action = recv.read_byte();

		Stage::get().get_summons().act(oid, action);
	}

	void DamageSummonHandler::handle(InPacket& recv) const
	{
		// v83 FUN_007a6ebe; the mob that hit it, if any, follows.
		recv.skip_int();
		int32_t oid = recv.read_int();
		recv.skip_byte();
		int32_t damage = recv.read_int();

		Stage::get().get_summons().hurt(oid, damage);
	}

	void RemoveReactorHandler::handle(InPacket& recv) const
	{
		// v83 FUN_0073551f. The state is the last one the reactor reached.
		int32_t oid = recv.read_int();
		int8_t state = recv.read_byte();
		recv.read_point();

		Stage::get().get_reactors().remove(oid, state);
	}
}
