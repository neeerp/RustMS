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
#include "SkillData.h"

#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <unordered_set>

namespace ms
{
	SkillData::SkillData(int32_t id)
	{
		/// Locate sources
		std::string strid = string_format::extend_id(id, 7);
		std::string jobid = strid.substr(0, 3);
		nl::node src = nl::nx::skill[jobid + ".img"]["skill"][strid];
		nl::node strsrc = nl::nx::string["Skill.img"][strid];

		// Load icons
		icons = {src["icon"], src["iconDisabled"], src["iconMouseOver"]};

		// Load strings
		name = std::string(strsrc["name"]);
		desc = std::string(strsrc["desc"]);

		for (int32_t level = 1; nl::node sub = strsrc["h" + std::to_string(level)]; level++)
			levels.emplace(level, sub);

		/// Load stats
		nl::node levelsrc = src["level"];

		for (auto sub : levelsrc)
		{
			float damage = (float) sub["damage"] / 100;
			int32_t matk = sub["mad"];
			int32_t fixdamage = sub["fixdamage"];
			int32_t mastery = sub["mastery"];
			uint8_t attackcount = (uint8_t) sub["attackCount"].get_integer(1);
			uint8_t mobcount = (uint8_t) sub["mobCount"].get_integer(1);
			uint8_t bulletcount = (uint8_t) sub["bulletCount"].get_integer(1);
			int16_t bulletcost = (int16_t) sub["bulletConsume"].get_integer(bulletcount);
			int32_t hpcost = sub["hpCon"];
			int32_t mpcost = sub["mpCon"];
			float chance = (float) sub["prop"].get_real(100.0) / 100;
			float critical = 0.0f;
			float ignoredef = 0.0f;
			float hrange = (float) sub["range"].get_real(100.0) / 100;
			Rectangle<int16_t> range = sub;
			int32_t hp = sub["hp"];
			int32_t mp = sub["mp"];
			int32_t time = sub["time"];
			int32_t level = string_conversion::or_default<int32_t>(sub.name(), -1);

			stats.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(level),
					std::forward_as_tuple(damage, matk, fixdamage, mastery, attackcount, mobcount, bulletcount,
										  bulletcost, hpcost, mpcost, chance, critical, ignoredef, hrange, range, hp, mp,
										  time)
			);
		}

		element = std::string(src["elemAttr"]);

		if (jobid == "900" || jobid == "910")
			reqweapon = Weapon::Type::NONE;
		else
			reqweapon = Weapon::by_value(100 + (int32_t) src["weapon"]);

		masterlevel = static_cast<int32_t>(stats.size());
		passive = (id % 10000) / 1000 == 0;
		flags = flags_of(id);
		invisible = src["invisible"].get_bool();

		/// Load required skills
		nl::node reqsrc = src["req"];

		for (auto sub : reqsrc)
		{
			int32_t skillid = string_conversion::or_default<int32_t>(sub.name(), -1);
			int32_t reqlv = sub.get_integer();

			reqskills.emplace(skillid, reqlv);
		}
	}

	int32_t SkillData::flags_of(int32_t id) const
	{
		// How v83's CUserLocal::DoActiveSkill dispatches each skill: to a
		// swing, a shot, or a spell. Every other skill is a special move.
		// Skills that charge first (Big Bang, Hurricane, Corkscrew Blow...)
		// aren't here; HeavenClient can't charge them yet. v83 casts Three
		// Snails as a spell, but RustMS reads it as a swing.
		static const std::unordered_set<int32_t> swings =
				{
				1000, 1009, 1020,
				1001004, 1001005,
				1111003, 1111004, 1111005, 1111006, 1111008,
				1121006, 1121008,
				1211002,
				1221007, 1221009, 1221011,
				1311001, 1311002, 1311003, 1311004, 1311005, 1311006,
				1321003,
				3101003,
				3201003,
				4001002, 4001334,
				4121008,
				4201004, 4201005,
				4211002, 4211004,
				4221001, 4221007,
				5001001, 5001002,
				5101002, 5101003,
				5111002, 5111004, 5111006,
				5121001, 5121004, 5121005, 5121007,
				5201004,
				5221003,
				9001001,
				10001000, 10001009, 10001020,
				11001002, 11001003,
				11111002, 11111003, 11111004, 11111006,
				14001002,
				15001001, 15001002,
				15101005,
				15111001, 15111003, 15111004,
				20000014, 20000015, 20000016, 20001000, 20001009, 20001020,
				21000002,
				21100001, 21100002,
				21110003, 21110006,
				21120005
				};
		static const std::unordered_set<int32_t> shots =
				{
				3001004, 3001005,
				3101005,
				3111003, 3111004, 3111006,
				3121003,
				3201005,
				3211003, 3211004, 3211006,
				3221003, 3221007,
				4001344,
				4101005,
				4111004, 4111005,
				4121003, 4121007,
				4221003,
				5001003,
				5121002,
				5201001, 5201006,
				5210000, 5211004, 5211005, 5211006,
				5220011, 5221009,
				11101004,
				13001003,
				13101005,
				13111000, 13111001, 13111006, 13111007,
				14001004,
				14101006,
				14111002, 14111005,
				15111007,
				21100004,
				21110004,
				21120006
				};
		// Shots that throw no ammo, as v83's FUN_007667ee decides.
		static const std::unordered_set<int32_t> ammo_free =
				{
				4111004, 4121003, 4221003, 5121002, 11101004, 13101005, 14101006, 15111007, 21100004, 21110004,
				21120006
				};
		static const std::unordered_set<int32_t> spells =
				{
				2001004, 2001005,
				2101004, 2101005,
				2111003, 2111006,
				2121003, 2121006, 2121007,
				2201004, 2201005,
				2211002, 2211003, 2211006,
				2221003, 2221006, 2221007,
				2301005,
				2311004,
				2321007, 2321008,
				12001003,
				12101002, 12101006,
				12111003, 12111005, 12111006
				};

		if (swings.count(id))
			return ATTACK;

		if (shots.count(id))
			return ammo_free.count(id) ? ATTACK | RANGED | NOAMMO : ATTACK | RANGED;

		if (spells.count(id))
			return ATTACK | MAGIC;

		return NONE;
	}

	bool SkillData::is_passive() const
	{
		return passive;
	}

	bool SkillData::is_attack() const
	{
		return flags & ATTACK;
	}

	bool SkillData::is_shot() const
	{
		return flags & RANGED;
	}

	bool SkillData::is_spell() const
	{
		return flags & MAGIC;
	}

	bool SkillData::uses_ammo() const
	{
		return (flags & RANGED) && !(flags & NOAMMO);
	}

	bool SkillData::is_invisible() const
	{
		return invisible;
	}

	int32_t SkillData::get_masterlevel() const
	{
		return masterlevel;
	}

	Weapon::Type SkillData::get_required_weapon() const
	{
		return reqweapon;
	}

	const SkillData::Stats &SkillData::get_stats(int32_t level) const
	{
		auto iter = stats.find(level);

		if (iter == stats.end())
		{
			static constexpr Stats null_stats = Stats(0.0f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0.0f, 0.0f, 0.0f, 0.0f,
													  Rectangle<int16_t>());

			return null_stats;
		}

		return iter->second;
	}

	const std::string &SkillData::get_name() const
	{
		return name;
	}

	const std::string &SkillData::get_desc() const
	{
		return desc;
	}

	const std::string &SkillData::get_level_desc(int32_t level) const
	{
		auto iter = levels.find(level);

		if (iter == levels.end())
		{
			static const std::string null_level = "Missing level description.";

			return null_level;
		} else
		{
			return iter->second;
		}
	}

	const Texture &SkillData::get_icon(Icon icon) const
	{
		return icons[icon];
	}

	const std::unordered_map<int32_t, int32_t> &SkillData::get_reqskills() const
	{
		return reqskills;
	}
}