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
#include "RegularAttack.h"

#include "AranCombo.h"

#include "../../Util/Randomizer.h"

#include <nlnx/nx.hpp>

namespace ms
{
	void RegularAttack::apply_useeffects(Char &) const
	{}

	std::string RegularAttack::apply_actions(Char &user, Attack::Type type) const
	{
		return action.apply(user, type);
	}

	void RegularAttack::apply_stats(const Char &user, Attack &attack) const
	{
		attack.damagetype = Attack::DMG_WEAPON;
		attack.skill = 0;
		attack.mobcount = static_cast<uint8_t>(
				aran_combo::plain_mob_count(user.get_job(), user.get_weapontype(), user.get_level()));
		attack.hitcount = 1;
		attack.stance = user.get_look().get_stance();

		if (attack.type == Attack::CLOSE)
			attack.range = user.get_afterimage().get_range();

		attack.blow.motion = physical_damage::motion(user.get_look().get_action_name(), attack.type == Attack::RANGED);
	}

	void RegularAttack::apply_hiteffects(const AttackUser &user, Mob &target) const
	{
		if (const Animation *spark = get_spark(user))
			target.show_hit(*spark, user.range, user.flip);
	}

	const Animation *RegularAttack::get_spark(const AttackUser &user) const
	{
		if (user.weapon / 1000000 != 1)
			return nullptr;

		// v83 sorts weapons into blades and blunts by their type.
		std::string name;

		switch (user.weapon / 10000 % 100)
		{
			case 30: // One-handed sword
			case 31: // One-handed axe
			case 33: // Dagger
			case 40: // Two-handed sword
			case 41: // Two-handed axe
			case 43: // Spear
			case 44: // Polearm
				name = "sword";
				break;
			case 32: // One-handed mace
			case 37: // Wand
			case 38: // Staff
			case 39:
			case 42: // Two-handed mace
			case 45: // Bow
			case 46: // Crossbow
			case 47: // Claw
			case 48: // Knuckle
			case 49: // Gun
				name = "mace";
				break;
			default:
				return nullptr;
		}

		// The last swing or stab of a combo gets the F set; any other hit
		// picks one of the two plain sets at random.
		switch (Stance::by_id(user.stance))
		{
			case Stance::Id::SWINGOF:
			case Stance::Id::SWINGTF:
			case Stance::Id::SWINGPF:
			case Stance::Id::STABOF:
			case Stance::Id::STABTF:
				name += "F";
				break;
			default:
				name += Randomizer().next_bool() ? "1" : "2";
				break;
		}

		auto iter = sparks.find(name);

		if (iter == sparks.end())
			iter = sparks.emplace(name, nl::nx::character["Afterimage"]["hit.img"][name]).first;

		return &iter->second;
	}

	Animation RegularAttack::get_bullet(const Char &user, int32_t bulletid) const
	{
		return bullet.get(user, bulletid);
	}

	bool RegularAttack::is_attack() const
	{
		return true;
	}

	bool RegularAttack::is_skill() const
	{
		return false;
	}

	int32_t RegularAttack::get_id() const
	{
		return 0;
	}

	SpecialMove::ForbidReason
	RegularAttack::can_use(int32_t, Weapon::Type, const Job &, uint16_t, uint16_t, uint16_t) const
	{
		// v83 never refuses a regular attack: a bow, crossbow, claw or gun
		// without ammo swings or stabs instead (Player::prepare_attack).
		return FBR_NONE;
	}
}