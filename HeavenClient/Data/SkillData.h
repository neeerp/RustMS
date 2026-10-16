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

#include "../Graphics/Texture.h"
#include "../Template/Cache.h"

#include "../Character/Inventory/Weapon.h"

namespace ms
{
	// Contains information about a skill
	class SkillData : public Cache<SkillData>
	{
	public:
		// The stats of one level
		struct Stats
		{
			float damage;
			int32_t matk;
			int32_t fixdamage;
			int32_t mastery;
			uint8_t attackcount;
			uint8_t mobcount;
			uint8_t bulletcount;
			int16_t bulletcost;
			int32_t hpcost;
			int32_t mpcost;
			float chance;
			float critical;
			float ignoredef;
			float hrange;
			Rectangle<int16_t> range;
			// HP and MP added to natural recovery, and Endure's seconds
			// between heals on a ladder or rope.
			int32_t hp;
			int32_t mp;
			int32_t time;
			// How far a shot reaches, from the same value as hrange; 0 if the
			// level has none.
			int16_t shotrange;

			constexpr Stats(float damage, int32_t matk, int32_t fixdamage, int32_t mastery, uint8_t attackcount,
							uint8_t mobcount, uint8_t bulletcount, int16_t bulletcost, int32_t hpcost, int32_t mpcost,
							float chance, float critical, float ignoredef, float hrange, Rectangle<int16_t> range,
							int32_t hp = 0, int32_t mp = 0, int32_t time = 0, int16_t shotrange = 0)
					: damage(damage), matk(matk), fixdamage(fixdamage), mastery(mastery), attackcount(attackcount),
					  mobcount(mobcount), bulletcount(bulletcount), bulletcost(bulletcost), hpcost(hpcost),
					  mpcost(mpcost), chance(chance), critical(critical), ignoredef(ignoredef), hrange(hrange),
					  range(range), hp(hp), mp(mp), time(time), shotrange(shotrange)
			{}
		};

		// Skill flags, taken from the v83 client
		enum Flags
		{
			NONE = 0x0000,
			ATTACK = 0x0001,
			RANGED = 0x0002,
			MAGIC = 0x0004,
			NOAMMO = 0x0008,
			ONEHANDED = 0x0010
		};

		// Icon types
		enum Icon
		{
			NORMAL,
			DISABLED,
			MOUSEOVER,
			NUM_ICONS
		};

		// Return whether the skill is passive
		bool is_passive() const;

		// Return whether the skill is an attack skill.
		bool is_attack() const;

		// Return whether the skill attacks with a shot.
		bool is_shot() const;

		// Return whether the skill attacks with a spell.
		bool is_spell() const;

		// Return whether the skill's shot uses up ammo.
		bool uses_ammo() const;

		// Return whether the skill needs a one-handed weapon, a wand or a staff.
		bool needs_onehanded() const;

		// Return whether this skill is invisible in the skill book ui.
		bool is_invisible() const;

		// Return the default masterlevel.
		int32_t get_masterlevel() const;

		// Return the required weapon.
		Weapon::Type get_required_weapon() const;

		// Return the stats of one level.
		// If there are no stats for that level, a default object is returned.
		const Stats &get_stats(int32_t level) const;

		// Return the name of the skill.
		const std::string &get_name() const;

		// Return the description of the skill.
		const std::string &get_desc() const;

		// Return the description of a level.
		// If there is no description for this level, a warning message is returned.
		const std::string &get_level_desc(int32_t level) const;

		// Return one of the skill icons.
		// Cannot fail if type is a valid enum.
		const Texture &get_icon(Icon icon) const;

		// Return id and level of all required skills.
		const std::unordered_map<int32_t, int32_t> &get_reqskills() const;

	private:
		// Allow the cache to use the constructor
		friend Cache<SkillData>;

		// Load a skill from the game files.
		SkillData(int32_t id);

		// Get some hard-coded information
		int32_t flags_of(int32_t id) const;

		std::unordered_map<int32_t, Stats> stats;
		std::string element;
		Weapon::Type reqweapon;
		int32_t masterlevel;
		int32_t flags;
		bool passive;
		bool invisible;

		std::string name;
		std::string desc;
		std::unordered_map<int32_t, std::string> levels;
		std::unordered_map<int32_t, int32_t> reqskills;

		std::array<Texture, Icon::NUM_ICONS> icons;
	};
}