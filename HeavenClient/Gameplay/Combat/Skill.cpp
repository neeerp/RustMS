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
#include "Skill.h"

#include "Heal.h"
#include "Prepare.h"

#include "../../Character/SkillId.h"
#include "../../Data/SkillData.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Skill::Skill(int32_t id) : skillid(id)
	{
		const SkillData &data = SkillData::get(skillid);

		std::string strid;

		if (skillid < 10000000)
			strid = string_format::extend_id(skillid, 7);
		else
			strid = std::to_string(skillid);

		nl::node src = nl::nx::skill[string_format::skill_img(skillid)]["skill"][strid];

		projectile = true;
		overregular = false;

		nl::node preparesrc = src["prepare"];
		prepare.action = std::string(preparesrc["action"]);
		prepare.time = preparesrc["time"];
		prepare.prepare = { preparesrc, preparesrc["z"] };
		prepare.keydown = { src["keydown"], src["keydown"]["z"] };
		prepare.keydownend = { src["keydownend"], src["keydownend"]["z"] };
		special = src["special"];
		affected = { src["affected"], src["affected"]["z"] };

		if (shot_rain::kind(skillid) != shot_rain::Kind::NONE || spell_area::kind(skillid) != spell_area::Kind::NONE)
		{
			nl::node specialsrc = src["special"];
			rain.params = {
				specialsrc["x"], specialsrc["y"], specialsrc["count"], specialsrc["fall"],
				specialsrc["interval"], specialsrc["start"], specialsrc["duration"]
			};

			for (nl::node drop = specialsrc["0"]; drop.size() > 0; drop = specialsrc[std::to_string(rain.drops.size())])
				rain.drops.emplace_back(drop);

			nl::node tilesrc = src["tile"];
			rain.distance = tilesrc["effectDistance"];

			for (nl::node flame = tilesrc["0"]; flame.size() > 0; flame = tilesrc[std::to_string(rain.flames.size())])
				rain.flames.emplace_back(flame);
		}

		sound = std::make_unique<SingleSkillSound>(strid);

		bool byleveleffect = src["CharLevel"]["10"]["effect"].size() > 0;
		bool multieffect = src["effect0"].size() > 0;

		if (byleveleffect)
		{
			useeffect = std::make_unique<ByLevelUseEffect>(src);
		} else if (multieffect)
		{
			useeffect = std::make_unique<MultiUseEffect>(src);
		} else
		{
			bool isanimation = src["effect"]["0"].data_type() == nl::node::type::bitmap;
			bool haseffect1 = src["effect"]["1"].size() > 0;

			if (isanimation)
			{
				useeffect = std::make_unique<SingleUseEffect>(src);
			} else if (haseffect1)
			{
				useeffect = std::make_unique<TwoHandedUseEffect>(src);
			} else
			{
				switch (skillid)
				{
					case SkillId::IRON_BODY:
					case SkillId::MAGIC_ARMOR:
						useeffect = std::make_unique<IronBodyUseEffect>();
						break;
					default:
						useeffect = std::make_unique<NoUseEffect>();
						break;
				}
			}
		}

		bool bylevelhit = src["CharLevel"]["10"]["hit"].size() > 0;
		bool byskilllevelhit = src["level"]["1"]["hit"].size() > 0;
		bool hashit0 = src["hit"]["0"].size() > 0;
		bool hashit1 = src["hit"]["1"].size() > 0;

		if (bylevelhit)
		{
			if (hashit0 && hashit1)
				hiteffect = std::make_unique<ByLevelTwoHandedHitEffect>(src);
			else
				hiteffect = std::make_unique<ByLevelHitEffect>(src);
		} else if (byskilllevelhit)
		{
			hiteffect = std::make_unique<BySkillLevelHitEffect>(src);
		} else if (hashit0 && hashit1)
		{
			hiteffect = std::make_unique<TwoHandedHitEffect>(src);
		} else if (hashit0)
		{
			hiteffect = std::make_unique<SingleHitEffect>(src);
		} else
		{
			hiteffect = std::make_unique<NoHitEffect>();
		}

		bool hasaction0 = src["action"]["0"].data_type() == nl::node::type::string;

		// A thrown grenade hits with no action, and flies as no bullet: it
		// shows its own effect where it lands (0x950921, 0x936528).
		bool thrown = prepare::throws(skillid);

		if (thrown)
		{
			action = std::make_unique<NoAction>();
		} else if (hasaction0)
		{
			action = std::make_unique<RandomAction>(src);
		} else if (data.is_attack())
		{
			bool bylevel = src["level"]["1"]["action"].data_type() == nl::node::type::string;

			if (bylevel)
			{
				action = std::make_unique<ByLevelAction>(src, skillid);
			} else
			{
				action = std::make_unique<RegularAction>();
				overregular = true;
			}
		} else
		{
			action = std::make_unique<NoAction>();
		}

		bool hasball = src["ball"].size() > 0;
		bool bylevelball = src["level"]["1"]["ball"].size() > 0;

		if (thrown)
		{
			bullet = std::make_unique<RegularBullet>();
			projectile = false;
		} else if (bylevelball)
		{
			bullet = std::make_unique<BySkillLevelBullet>(src, skillid);
		} else if (hasball)
		{
			bullet = std::make_unique<SingleBullet>(src);
		} else
		{
			bullet = std::make_unique<RegularBullet>();
			projectile = false;
		}
	}

	void Skill::apply_useeffects(Char &user) const
	{
		useeffect->apply(user);

		sound->play_use();
	}

	void Skill::play_use() const
	{
		sound->play_use();
	}

	void Skill::play_use2() const
	{
		sound->play_use2();
	}

	void Skill::play_loop() const
	{
		sound->play_loop();
	}

	void Skill::stop_loop() const
	{
		sound->stop_loop();
	}

	std::string Skill::apply_actions(Char &user, Attack::Type type) const
	{
		return action->apply(user, type);
	}

	void Skill::apply_stats(const Char &user, Attack &attack) const
	{
		attack.skill = skillid;

		int32_t level = user.get_skilllevel(skillid);
		const SkillData::Stats stats = SkillData::get(skillid).get_stats(level);

		if (stats.fixdamage)
		{
			attack.fixdamage = stats.fixdamage;
			attack.damagetype = Attack::DMG_FIXED;
		} else if (skillid == heal::ID)
		{
			attack.heal = stats.hp;
			attack.element = SkillData::get(skillid).get_element();
			attack.damagetype = Attack::DMG_MAGIC;
		} else if (stats.matk)
		{
			attack.matk += stats.matk;
			attack.mastery = stats.mastery;
			attack.element = SkillData::get(skillid).get_element();
			attack.damagetype = Attack::DMG_MAGIC;
		} else
		{
			attack.mindamage *= stats.damage;
			attack.maxdamage *= stats.damage;
			attack.damagetype = Attack::DMG_WEAPON;
		}

		attack.critical += stats.critical;
		attack.ignoredef += stats.ignoredef;
		attack.mobcount = stats.mobcount;
		attack.hrange = stats.hrange;
		attack.shotrange = stats.shotrange;
		attack.bulletcost = stats.bulletcost;

		switch (attack.type)
		{
			case Attack::RANGED:
				attack.hitcount = stats.bulletcount;
				break;
			default:
				attack.hitcount = stats.attackcount;
				break;
		}

		if (!stats.range.empty())
			attack.range = stats.range;

		if (projectile && !attack.bullet)
		{
			switch (skillid)
			{
				case SkillId::THREE_SNAILS:
					switch (level)
					{
						case 1:
							attack.bullet = 4000019;
							break;
						case 2:
							attack.bullet = 4000000;
							break;
						case 3:
							attack.bullet = 4000016;
							break;
					}
					break;
				default:
					attack.bullet = skillid;
					break;
			}
		}

		if (overregular)
		{
			attack.stance = user.get_look().get_stance();

			if (attack.type == Attack::CLOSE && !projectile)
				attack.range = user.get_afterimage().get_range();
		}
	}

	void Skill::apply_hiteffects(const AttackUser &user, Mob &target) const
	{
		hiteffect->apply(user, target);

		sound->play_hit();
	}

	Animation Skill::get_bullet(const Char &user, int32_t bulletid) const
	{
		return bullet->get(user, bulletid);
	}

	Animation Skill::get_ball(size_t kind) const
	{
		return bullet->get_kind(kind);
	}

	bool Skill::has_ball() const
	{
		return projectile;
	}

	bool Skill::is_attack() const
	{
		return SkillData::get(skillid).is_attack();
	}

	bool Skill::is_skill() const
	{
		return true;
	}

	Animation Skill::get_special() const
	{
		return special;
	}

	const SkillPrepare::Effect &Skill::get_affected() const
	{
		return affected;
	}

	const SkillRain &Skill::get_rain() const
	{
		return rain;
	}

	const SkillPrepare &Skill::get_prepare() const
	{
		return prepare;
	}

	int32_t Skill::get_id() const
	{
		return skillid;
	}

	SpecialMove::ForbidReason
	Skill::can_use(int32_t level, Weapon::Type weapon, const Job &job, uint16_t hp, uint16_t mp, uint16_t bullets) const
	{
		if (level <= 0 || level > SkillData::get(skillid).get_masterlevel())
			return FBR_OTHER;

		if (job.can_use(skillid) == false)
			return FBR_OTHER;

		const SkillData::Stats stats = SkillData::get(skillid).get_stats(level);

		if (hp <= stats.hpcost)
			return FBR_HPCOST;

		if (mp < stats.mpcost)
			return FBR_MPCOST;

		if (SkillData::get(skillid).needs_onehanded())
		{
			switch (weapon)
			{
				case Weapon::SWORD_1H:
				case Weapon::AXE_1H:
				case Weapon::MACE_1H:
				case Weapon::DAGGER:
				case Weapon::WAND:
				case Weapon::STAFF:
					break;
				default:
					return FBR_ONEHANDED;
			}
		}

		Weapon::Type reqweapon = SkillData::get(skillid).get_required_weapon();

		if (weapon != reqweapon && reqweapon != Weapon::NONE)
			return FBR_WEAPONTYPE;

		if (!SkillData::get(skillid).uses_ammo())
			return FBR_NONE;

		switch (weapon)
		{
			case Weapon::BOW:
			case Weapon::CROSSBOW:
			case Weapon::CLAW:
			case Weapon::GUN:
				return (bullets >= stats.bulletcost) ? FBR_NONE : FBR_BULLETCOST;
			default:
				return FBR_NONE;
		}
	}
}