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

#include "SummonRules.h"

#include "../MovePath.h"
#include "../Combat/DamageNumber.h"
#include "../Physics/Physics.h"

#include "../../Graphics/Animation.h"
#include "../../Util/Randomizer.h"

#include <list>
#include <map>

namespace ms
{
	// A user's summon: a Puppet, Silver Hawk, the Octopus, Elquines...
	// (v83 CSummoned, report 158). The owner's client moves it and decides
	// when it attacks; the others play what the server passes on.
	class Summon
	{
	public:
		Summon(int32_t owner, int32_t skill, uint8_t level, Point<int16_t> position, uint8_t action, uint8_t moves,
			   uint8_t assist, bool appear, bool local);

		// Play an animation once, over the move action.
		void play(summon_rules::Action action);

		// Play its path, as its owner's client sent it (0xB1).
		void receive(const std::vector<Movement> &elements);

		// A mob hurt it: it blinks for 1.5 s, and flinches if hurt
		// (FUN_007a6ebe).
		void hurt(int32_t damage);

		// Go, playing `die` first if `dying`. Whether anything is left to
		// show.
		bool remove(bool dying);

		// Move and animate it one step. The owner's own summon follows
		// `owner` (held on a ladder or rope), and returns the path to send
		// once there is one. Whether it is gone is `gone()`.
		std::optional<move_path::Path> update(const Physics &physics, Point<int16_t> owner, bool held,
											  Randomizer &random);

		void draw(double viewx, double viewy, float alpha) const;

		bool gone() const;
		int32_t get_owner() const;
		int32_t get_skill() const;
		uint8_t get_level() const;
		uint8_t get_moves() const;
		uint8_t get_assist() const;
		Point<int16_t> get_position() const;
		bool is_left() const;
		bool is_local() const;
		bool is_flying() const;
		int8_t get_layer() const;
		// Whether it may attack: it can act, and plays nothing.
		bool is_idle() const;
		// When it last attacked, in the combat's ms.
		int64_t last_attack = 0;

		void set_left(bool left);

	private:
		const Animation &animation_for(summon_rules::Action action);
		void show(summon_rules::Action action);
		// The move action it shows: its stance times two, plus one facing
		// left.
		uint8_t move_action() const;

		int32_t owner;
		int32_t skill;
		uint8_t level;
		uint8_t moves;
		uint8_t assist;
		bool local;

		double x;
		double y;
		bool left;
		uint8_t action;

		std::map<int8_t, Animation> animations;
		summon_rules::Action showing = summon_rules::NONE;
		summon_rules::Action oneshot = summon_rules::NONE;
		bool dying = false;
		bool removed = false;
		// The ms left of its blink after a hit; minus after a miss.
		int16_t hitcountdown = 0;

		// Where it stands, for the layer it shows on.
		PhysicsObject phobj;

		// The owner's: how it moves, and the path not yet sent.
		summon_rules::Follower follower;
		summon_rules::Circler circler;
		Point<int16_t> anchor;
		move_path::Recorder recorder;
		// The others': the paths to play.
		move_path::Player shown;
	};

	// The summons on the map, by object id.
	class MapSummons
	{
	public:
		// SPAWN_SUMMON (0xAF). The player's own summon whose id is known
		// is kept as it is; another user's starts over (0x95adec, 0x938f61).
		void spawn(int32_t owner, int32_t oid, int32_t skill, uint8_t level, Point<int16_t> position, uint8_t action,
				   uint8_t moves, uint8_t assist, bool appear, bool local);

		// REMOVE_SUMMON (0xB0): reasons 0, 3, and 4, and 2 for a Puppet,
		// play `die` first. The skill it was, or 0 if there was none.
		int32_t remove(int32_t oid, uint8_t reason);

		// Drop a user's summons without a trace, as they leave the map.
		void remove_owner(int32_t owner);

		// MOVE_SUMMON (0xB1).
		void move(int32_t oid, const std::vector<Movement> &elements);

		// SUMMON_ATTACK (0xB2) and SUMMON_SKILL (0xB3): show its animation.
		void act(int32_t oid, uint8_t action);

		// DAMAGE_SUMMON (0xB4): its number and its blink.
		void hurt(int32_t oid, int32_t damage);

		// Move and animate them; the player's own follow `owner` and send
		// their paths.
		void update(const Physics &physics, Point<int16_t> owner, bool held);

		// The summons on a layer of the map that stand on it, or the fliers
		// above all (`layer` -1).
		void draw(int8_t layer, double viewx, double viewy, float alpha) const;

		void clear();

		// The player's own summons, for their attacks.
		std::vector<std::pair<int32_t, Summon *>> get_local();

		Summon *get(int32_t oid);

	private:
		std::map<int32_t, Summon> summons;
		std::list<DamageNumber> numbers;
		Randomizer random;
	};
}
