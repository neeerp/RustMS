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

#include "PetFollow.h"

#include "../Pets.h"
#include "../StyledTag.h"

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

		// Stand in its clothes at `pos` in a window, without its name tag,
		// as the Character Info window shows it (FUN_0090433a).
		void draw_standing(Point<int16_t> pos, float alpha) const;

		void update_standing();

		// Follow the owner at `charpos`, facing left if `owner_left`,
		// returning a path to send once there is one, or play the paths
		// received.
		std::optional<move_path::Path> update(const Physics &physics, Point<int16_t> charpos, bool owner_left);

		// MOVE_PET (170): where another's pet went.
		void send_movement(const std::vector<Movement> &movements);

		// What a pet does and says, as PET_CHAT (0xA8) sends it.
		struct Said
		{
			// 0 a command's answer, 1 food's, 2 a `/pet` line, 3 one of its
			// own.
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

		// `/pet <text>`: the pet at `level` says `text`, or does its slang
		// if the curse filter `censored` it (pets::speech). Shows it and
		// returns what to send, always action 0 (FUN_007050be).
		std::optional<Said> say(const std::string &text, bool censored, bool styled, int16_t level);

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

		Point<int16_t> get_position() const;

		bool is_on_ground() const;

		const PhysicsObject &get_phobj() const;

		// PET_NAMECHANGE (172): the pet answers to its new name, and shows
		// its kind's name tag if `name_tag`.
		void set_name(const std::string &name, bool name_tag);

		// Its owner's client shows its kind's name tag while a label ring
		// is worn for it (FUN_00705e2c).
		void set_name_tag(bool name_tag);

		// Dress it in the clothes `wear` (180xxxx) its owner's look gives
		// its place in the list, or none for 0 (FUN_00703d15).
		void set_wear(int32_t wear);

		// PET_EXCEPTION_LIST (173): what it leaves while it wears an Item
		// Ignore Pendant, by name (pets::sort_ignored).
		void set_ignored(std::vector<int32_t> ids);

		const std::vector<int32_t> &get_ignored() const;

		// Which way it walks toward a drop in Wing Boots: -1 left, 1 right,
		// 0 none (FUN_00508268).
		void set_sweep(int8_t direction);
		// Its place among its owner's pets (petfollow::role).
		void set_role(uint8_t role);
		// Whether it wears a Binocular, and so strays farther.
		void set_long_range(bool longrange);

		bool is_local() const;

		Stance get_stance() const;

	private:
		// Its stance as v83 sends it: the move action times two, plus one
		// facing left.
		uint8_t get_stancebyte() const;

		// Count how long it has stood at `before`, end what it was showing
		// once it moves, and now and then act on its own.
		void update_acting(Point<int16_t> before);

		// Walk toward its spot near the owner at `charpos`, jumping up to
		// them or over a wall, with now and then a run.
		void follow(const Physics &physics, Point<int16_t> charpos, const petfollow::Goal &goal, bool owner_left);

		// Its name tag: its kind's style (info/nameTag) when shown, else
		// the plain box (FUN_005f0334 type 0x3eb).
		void build_name_tag();

		// Draw the clothes' frame for the one `animation` shows.
		void draw_dressing(const std::vector<Texture> &textures, const Animation &animation, const DrawArgument &args,
						   float alpha) const;

		int32_t itemid;
		std::string name;
		int64_t serial;
		std::vector<int32_t> ignored;
		bool local;
		Stance stance;
		// Facing right.
		bool flip;
		// Which way it walks toward a drop, 0 for none.
		int8_t sweep = 0;
		// Its spot near its owner, and how long it has stayed put.
		int32_t goalx = 0;
		int32_t owner_still = 0;
		// How it follows on the ground (petfollow): its role, the way it
		// walks and the way it steers, ms toward its next step, its x at
		// the last one, how long it has been running (-1 when not) and
		// the distance a dash adds.
		uint8_t role = 0;
		bool longrange = false;
		int8_t held = 0;
		int8_t steer = 0;
		int32_t thinking = 0;
		int32_t lastx = 0;
		int32_t ran = -1;
		int32_t bonus = 0;

		EnumMap<Stance, Animation> animations;
		PhysicsObject phobj;
		bool name_tag;
		StyledTag nametag;

		// The owner's: the path not yet sent.
		move_path::Recorder recorder;
		// The others': the paths to play.
		move_path::Player shown;

		// The action it was told to show (-1 for none) until it ends or the
		// pet moves.
		int16_t action;
		Animation acting;
		std::string acting_name;

		// The clothes it wears, and their frames over each of its own
		// (Character/PetEquip/<wear>.img/<itemid>/<animation>).
		int32_t wear;
		EnumMap<Stance, std::vector<Texture>> dressing;
		std::vector<Texture> acting_dressing;
		ChatBalloon balloon;
		// How far above its feet the balloon's arrow points: its frame's
		// origin when it spoke, plus 5 (FUN_0048b448).
		int16_t balloon_lift;
		Animation effect;
		bool effecting;
		// A command sent and not yet answered (+0xb4).
		bool pending;
		// How long it has stood still, in ms (+0xb8).
		int32_t still;
	};
}