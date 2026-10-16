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

#include "../Template/Rectangle.h"

#include <cstdint>
#include <vector>
#include <unordered_map>

namespace ms
{
	struct Attack
	{
		enum Type
		{
			CLOSE,
			RANGED,
			MAGIC
		};

		enum DamageType
		{
			DMG_WEAPON,
			DMG_MAGIC,
			DMG_FIXED
		};

		Type type = Type::CLOSE;
		DamageType damagetype = DamageType::DMG_WEAPON;

		double mindamage = 1.0;
		double maxdamage = 1.0;
		float critical = 0.0f;
		float ignoredef = 0.0f;
		int32_t matk = 0;
		int32_t accuracy = 0;
		int32_t fixdamage = 0;
		int16_t playerlevel = 1;
		// What a spell's damage comes from: the player's magic, INT and LUK,
		// the spell's mastery and element, and Element Amplification's percent.
		int32_t magic = 0;
		int32_t intelligence = 0;
		int32_t luck = 0;
		int32_t mastery = 0;
		int32_t amplify = 100;
		uint8_t element = 0;

		uint8_t hitcount = 0;
		uint8_t mobcount = 0;
		uint8_t speed = 0;
		uint8_t stance = 0;
		int32_t skill = 0;
		// How far the skill's shot reaches, if it says.
		int16_t shotrange = 0;
		int32_t bullet = 0;
		// Ammo a shot uses up, and the USE slot it comes from.
		int16_t bulletcost = 1;
		int16_t bulletslot = 0;

		Point<int16_t> origin;
		Rectangle<int16_t> range;
		float hrange = 1.0f;
		bool toleft = false;
	};

	struct MobAttack
	{
		Attack::Type type = Attack::Type::CLOSE;
		int32_t watk = 0;
		int32_t matk = 0;
		int32_t mobid = 0;
		int32_t oid = 0;
		Point<int16_t> origin;
		bool valid = false;

		// Create a mob attack for touch damage.
		MobAttack(int32_t watk, Point<int16_t> origin, int32_t mobid, int32_t oid) : type(Attack::Type::CLOSE),
																					 watk(watk), origin(origin),
																					 mobid(mobid), oid(oid), valid(true)
		{}

		MobAttack() : valid(false)
		{}

		explicit operator bool() const
		{
			return valid;
		}
	};

	struct MobAttackResult
	{
		int32_t damage;
		int32_t mobid;
		int32_t oid;
		uint8_t direction;

		MobAttackResult(const MobAttack &attack, int32_t damage, uint8_t direction) : damage(damage),
																					  direction(direction),
																					  mobid(attack.mobid),
																					  oid(attack.oid)
		{}
	};

	struct AttackResult
	{
		AttackResult(const Attack &attack)
		{
			type = attack.type;
			hitcount = attack.hitcount;
			skill = attack.skill;
			speed = attack.speed;
			stance = attack.stance;
			bullet = attack.bullet;
			bulletslot = attack.bulletslot;
			toleft = attack.toleft;
		}

		AttackResult()
		{}

		Attack::Type type;
		int32_t attacker = 0;
		uint8_t mobcount = 0;
		uint8_t hitcount = 1;
		int32_t skill = 0;
		int32_t charge = 0;
		int32_t bullet = 0;
		int16_t bulletslot = 0;
		uint8_t level = 0;
		uint8_t display = 0;
		uint8_t stance = 0;
		uint8_t speed = 0;
		bool toleft = false;
		// Each target's oid and damage, in target order.
		std::vector<std::pair<int32_t, std::vector<std::pair<int32_t, bool>>>> damagelines;
		int32_t first_oid;
		int32_t last_oid;
	};

	struct AttackUser
	{
		int32_t skilllevel;
		uint16_t level;
		bool secondweapon;
		bool flip;
		int32_t weapon;
		uint8_t stance;
		// Where the attack reached, in map coordinates.
		Rectangle<int16_t> range;
	};
}