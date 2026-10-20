#pragma once

#include <cstdint>

#include "../../Character/Inventory/Weapon.h"

namespace ms
{
	// Whether a skill may be used with the equipped weapon, as v83's
	// DoActiveSkill and its handlers decide.
	namespace skill_weapon
	{
		enum Refusal
		{
			FITS,
			// "You cannot use this skill with the weapon you're equipped
			// with." (0xb1c)
			WEAPON,
			// "This skill cannot be used with the equipped item." (0x1116)
			EQUIPPED,
			// Refused without a word.
			QUIET
		};

		// The shots FUN_009696a2 fires whatever the weapon.
		inline bool any_weapon(int32_t skill)
		{
			switch (skill)
			{
			case 4121003: // Taunt
			case 4221003: // Taunt
			case 5121002: // Energy Orb
			case 11101004: // Soul Blade
			case 15111007: // Shark Wave
			case 21100004: // Combo Smash
			case 21110004: // Combo Penril
			case 21120006: // Combo Tempest
				return true;
			default:
				return false;
			}
		}

		// The weapons FUN_00766157 lets a skill be used with, given the one
		// its Skill.img names.
		inline bool fits(int32_t skill, Weapon::Type weapon, Weapon::Type required)
		{
			switch (skill)
			{
			case 5001001: // Flash Fist
			case 15001001: // Straight
				return weapon == Weapon::KNUCKLE || weapon == Weapon::GUN;
			case 5101002: // Backspin Blow
			case 5101003: // Double Uppercut
			case 5101004: // Corkscrew Blow
			case 5111002: // Energy Blast
			case 5111006: // Shockwave
			case 5121001: // Dragon Strike
			case 5121002: // Energy Orb
			case 5121004: // Demolition
			case 5121005: // Snatch
			case 5121007: // Barrage
			case 15101003: // Corkscrew Blow
			case 15101005: // Energy Blast
			case 15111003: // Shockwave
			case 15111004: // Barrage
				return weapon == Weapon::KNUCKLE;
			case 11101004: // Soul Blade
			case 11111006: // Soul Driver
				return weapon == Weapon::SWORD_1H || weapon == Weapon::SWORD_2H;
			// A sword's charges and a mace's, and Charged Blow with either
			// (0x967ac1).
			case 1211003:
			case 1211005:
			case 1211007:
			case 1221003:
				return weapon == Weapon::SWORD_1H || weapon == Weapon::SWORD_2H;
			case 1211004:
			case 1211006:
			case 1211008:
			case 1221004:
				return weapon == Weapon::MACE_1H || weapon == Weapon::MACE_2H;
			case 1211002:
				return weapon == Weapon::SWORD_1H || weapon == Weapon::SWORD_2H || weapon == Weapon::MACE_1H ||
					weapon == Weapon::MACE_2H;
			case 12101006: // Fire Pillar
			case 12111005: // Flame Gear
			case 12111006: // Fire Strike
				return weapon == Weapon::WAND || weapon == Weapon::STAFF;
			default:
				return required == Weapon::NONE || weapon == required;
			}
		}

		// shot: whether FUN_009696a2 fires the skill.
		inline Refusal check(int32_t skill, bool shot, Weapon::Type weapon, Weapon::Type required)
		{
			bool wrong = false;

			switch (skill)
			{
			// FUN_00950921 swings Brandish only with a sword or an axe.
			case 1121008:
			case 11111004:
				wrong = weapon != Weapon::SWORD_1H && weapon != Weapon::AXE_1H && weapon != Weapon::SWORD_2H &&
					weapon != Weapon::AXE_2H;
				break;
			// The held skills (0x96a86e).
			case 5101004: // Corkscrew Blow
			case 15101003:
				wrong = weapon != Weapon::KNUCKLE;
				break;
			case 5201002: // Grenade
				wrong = weapon != Weapon::GUN;
				break;
			case 14111006: // Poison Bomb
				wrong = weapon != Weapon::CLAW;
				break;
			case 3121004: // Hurricane
			case 3221001: // Piercing Arrow
			case 5221004: // Rapid Fire
			case 13111002: // Hurricane
				if (weapon != Weapon::BOW && weapon != Weapon::CROSSBOW && weapon != Weapon::GUN)
					return QUIET;

				break;
			default:
				// A shot needs the weapon of its job, Skill.img aside.
				if (shot && !any_weapon(skill))
				{
					switch ((skill / 10000) % 1000 / 100)
					{
					case 3:
						wrong = weapon != Weapon::BOW && weapon != Weapon::CROSSBOW;
						break;
					case 4:
						wrong = weapon != Weapon::CLAW;
						break;
					case 5:
						wrong = weapon != Weapon::GUN;
						break;
					default:
						wrong = true;
						break;
					}
				}

				break;
			}

			if (wrong)
				return WEAPON;

			return fits(skill, weapon, required) ? FITS : EQUIPPED;
		}
	}
}
