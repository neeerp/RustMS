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

#include "../Pets.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../IO/Components/ChatBalloon.h"
#include "../../Template/EnumMap.h"

#include "../../Gameplay/MovePath.h"
#include "../../Gameplay/Physics/Physics.h"
#include "../../Net/Handlers/Helpers/PetParser.h"

#include <optional>

namespace ms
{
	class PetLook
	{
	public:
		enum Stance : uint8_t
		{
			MOVE,
			STAND,
			JUMP,
			ALERT,
			PRONE,
			FLY,
			HANG,
			WARP,
			LENGTH
		};

		// The animation a v83 move action shows, of those HC loads.
		static Stance stancebyvalue(uint8_t value);

		// The player's own pet (`local`) follows them and says where it
		// went; another's goes where its owner's client says.
		PetLook(const PetEntry &entry, bool local);

		PetLook();

		void draw(double viewx, double viewy, float alpha) const;

		// Follow the owner at `charpos`, returning a path to send once
		// there is one, or play the paths received.
		std::optional<move_path::Path> update(const Physics &physics, Point<int16_t> charpos);

		// MOVE_PET (170): where another's pet went.
		void send_movement(const std::vector<Movement> &movements);

		// What a pet does and says, as PET_CHAT (0xA8) sends it.
		struct Said
		{
			// 0 a command's answer, 1 food's, 3 one of its own.
			uint8_t type;
			int16_t action;
			std::string text;
		};

		// Show `action` (pets::action) and say `text` in a balloon, in the
		// kind's style when `styled` (FUN_007055e2). False for an action it
		// doesn't have, which shows and sends nothing.
		bool act(const Said &said, bool styled);

		// The command the owner's chat line gives it at `level`, marking an
		// answer pending (FUN_00704d5d). It only listens standing still,
		// between commands and actions.
		std::optional<pets::Heard> hear(const std::string &line, int16_t level);

		// PET_COMMAND (174): whether it obeyed `command`. Picks an answer and
		// shows it, returning it to send (FUN_007048ab).
		std::optional<Said> answer(uint8_t command, bool obeyed, bool styled, int16_t level);

		// PET_COMMAND (174) for food: whether it enjoyed it. Picks the food
		// answer for its level, shows it and returns it to send.
		std::optional<Said> eat(bool enjoyed, bool styled, int16_t level);

		// Plays `Sound.nx Pet.img/<id>/eat`, if it has one.
		void play_eat_sound() const;

		// User effect 4 on it (FUN_0070537d): 0 it levelled up, 1 teleport,
		// 2 its warp, 3 evolution.
		void show_effect(uint8_t kind);

		void set_position(int16_t xpos, int16_t ypos);

		void set_stance(Stance stance);

		void set_stance(uint8_t stancebyte);

		int32_t get_itemid() const;

		int64_t get_serial() const;

		// PET_NAMECHANGE (172): the pet answers to its new name.
		void set_name(const std::string &name);

		bool is_local() const;

		Stance get_stance() const;

	private:
		// Its stance as v83 sends it: the move action times two, plus one
		// facing left.
		uint8_t get_stancebyte() const;

		// Count how long it has stood at `before`, end what it was showing
		// once it moves, and now and then act on its own.
		void update_acting(Point<int16_t> before);

		int32_t itemid;
		std::string name;
		int64_t serial;
		bool local;
		Stance stance;
		// Facing right.
		bool flip;

		EnumMap<Stance, Animation> animations;
		PhysicsObject phobj;
		Text namelabel;

		// The owner's: the path not yet sent.
		move_path::Recorder recorder;
		// The others': the paths to play.
		move_path::Player shown;

		// The action it was told to show (-1 for none) until it ends or the
		// pet moves.
		int16_t action;
		Animation acting;
		ChatBalloon balloon;
		Animation effect;
		bool effecting;
		// A command sent and not yet answered (+0xb4).
		bool pending;
		// How long it has stood still, in ms (+0xb8).
		int32_t still;
	};
}