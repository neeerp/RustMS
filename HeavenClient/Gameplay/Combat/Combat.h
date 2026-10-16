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

#include <cstdint>

#include "RegularAttack.h"

#include "../MapleMap/MapChars.h"
#include "../MapleMap/MapMobs.h"
#include "../MapleMap/MapReactors.h"

#include "../../Character/Player.h"
#include "../../Template/TimedQueue.h"

namespace ms
{
	class Combat
	{
	public:
		Combat(Player &player, MapChars &chars, MapMobs &mobs, MapReactors &reactors);

		// Draw bullets, damage numbers etc.
		void draw(double viewx, double viewy, float alpha) const;

		// Poll attacks, damage effects, etc.
		void update();

		// Make the player use a special move
		void use_move(int32_t move_id);

		// Add an attack to the attack queue.
		void push_attack(const AttackResult &attack);

		// Show a buff effect.
		void show_buff(int32_t cid, int32_t skillid, int8_t level);

		// Show a buff effect.
		void show_player_buff(int32_t skillid);

	private:
		struct DamageEffect
		{
			AttackUser user;
			DamageNumber number;
			int32_t damage;
			bool toleft;
			int32_t target_oid;
			int32_t move_id;
		};

		struct BulletEffect
		{
			DamageEffect damageeffect;
			Bullet bullet;
			Point<int16_t> target;
		};

		void apply_attack(const AttackResult &attack);

		void apply_move(const SpecialMove &move);

		// The melee stance a regular ranged attack whacks a mob right in
		// front of the player with, or NONE to shoot.
		Stance::Id find_whack(const Attack &attack) const;

		std::vector<int32_t>
		find_closest(MapObjects *objs, Rectangle<int16_t> range, Point<int16_t> origin, uint8_t objcount,
					 bool use_mobs) const;

		void apply_use_movement(const SpecialMove &move);

		void apply_result_movement(const SpecialMove &move, const AttackResult &result);

		void apply_rush(const AttackResult &result);

		void apply_bullet_effect(const BulletEffect &effect);

		void apply_damage_effect(const DamageEffect &effect);

		// The area an attack covers, from the attacker's position and facing.
		static Rectangle<int16_t> place_range(Point<int16_t> origin, Rectangle<int16_t> range, float hrange,
											  bool toleft);

		void extract_effects(const Char &user, const SpecialMove &move, const AttackResult &result,
							 Rectangle<int16_t> range);

		// Tell the targets how many hits are coming (sign 1) or cancel them (sign -1),
		// so a mob the server kills waits for the hits before dying.
		void expect_hits(const AttackResult &result, int32_t sign);

		std::vector<DamageNumber> place_numbers(const std::vector<std::pair<int32_t, bool>> &damagelines, int32_t move_id);

		const SpecialMove &get_move(int32_t move_id);

		Player &player;
		MapChars &chars;
		MapMobs &mobs;
		MapReactors &reactors;

		std::unordered_map<int32_t, Skill> skills;
		RegularAttack regularattack;

		TimedQueue<AttackResult> attackresults;
		TimedQueue<BulletEffect> bulleteffects;
		TimedQueue<DamageEffect> damageeffects;

		std::list<BulletEffect> bullets;
		std::list<DamageNumber> damagenumbers;
	};
}