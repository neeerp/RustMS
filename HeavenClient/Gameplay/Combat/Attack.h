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
#include <optional>
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
			DMG_FIXED,
			// Shadow Meso's, from the mesos it throws.
			DMG_MESO
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
		// Shadow Meso's `x`, what a critical adds in percent.
		int32_t mesocritical = 0;
		int16_t playerlevel = 1;
		// What a spell's damage comes from: the player's magic, INT and LUK,
		// the spell's mastery and element, and Element Amplification's percent.
		int32_t magic = 0;
		int32_t intelligence = 0;
		int32_t luck = 0;
		int32_t mastery = 0;
		int32_t amplify = 100;
		uint8_t element = 0;
		// The weapon's percent for that element (an Elemental Wand or Staff).
		int32_t elemboost = 100;
		// Heal's `hp` percent, which it hurts the undead with in place of a
		// magic attack, and how many it reaches, mobs and party members.
		int32_t heal = 0;
		int32_t healed = 0;

		uint8_t hitcount = 0;
		// Shadow Partner's second set of bullets makes 2.
		uint8_t sets = 1;
		uint8_t mobcount = 0;
		uint8_t speed = 0;
		uint8_t stance = 0;
		int32_t skill = 0;
		// How long a held skill was charged, in ms.
		int32_t charge = 0;
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
		// The mob's level and accuracy, which the player's avoid is weighed
		// against.
		int32_t level = 0;
		int32_t accuracy = 0;
		// What Power Guard weighs against a touch: the mob's avoid, full HP
		// and fixedDamage, and whether it is a boss or invincible.
		int32_t avoid = 0;
		int32_t maxhp = 0;
		int32_t fixeddamage = 0;
		bool boss = false;
		bool invincible = false;
		// The percent a Power Up or Magic Up puts its physical or magic
		// hits at, or 0 without one.
		int32_t powerup = 0;
		int32_t magicup = 0;
		Point<int16_t> origin;
		// Which of the mob's attacks (attack1 is 0) hit, or -1 for its body.
		int8_t index = -1;
		bool magic = false;
		// A magic attack's element: 0 none, 1 ice, 2 fire, 3 lightning,
		// 4 poison (FUN_006811a7).
		uint8_t element = 0;
		// A deadly attack leaves 1 HP, and an MP burn takes 1.
		bool deadly = false;
		bool mpburn = false;
		// An attack knocks the player the way the mob faced; a touch, away
		// from its body.
		std::optional<bool> pushleft;
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
		int8_t index;
		uint8_t element;
		// Whether Power Stance kept the player in place.
		bool stance;
		// Power Guard's percent on a touch or Mana Reflection's on a spell,
		// or 0, and the number it shows over the mob (0 a miss).
		uint8_t reflect = 0;
		int32_t reflected = 0;
		// Where the mob and the player were.
		Point<int16_t> mobposition;
		Point<int16_t> position;

		MobAttackResult(const MobAttack &attack, int32_t damage, uint8_t direction, bool stance) :
			damage(damage), mobid(attack.mobid), oid(attack.oid), direction(direction), index(attack.index),
			element(attack.magic ? attack.element : 0), stance(stance), mobposition(attack.origin)
		{}
	};

	struct AttackResult
	{
		AttackResult(const Attack &attack)
		{
			type = attack.type;
			hitcount = attack.hitcount;
			sets = attack.sets;
			skill = attack.skill;
			charge = attack.charge;
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
		uint8_t sets = 1;
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