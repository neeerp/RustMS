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
#include "SkillBook.h"

#include "../Data/SkillData.h"
#include "../Gameplay/Combat/AranCombo.h"
#include "../Gameplay/Combat/PqSkills.h"

namespace ms
{
	void SkillBook::set_skill(int32_t id, int32_t level, int32_t mlevel, int64_t expire)
	{
		skillentries[id] = {level, mlevel, expire};
	}

	bool SkillBook::has_skill(int32_t id) const
	{
		return skillentries.count(id) > 0;
	}

	int32_t SkillBook::get_level(int32_t id) const
	{
		// The secret skills are always at level 1 (FUN_007616f6).
		if (pq_skills::kind(id) != pq_skills::Kind::NONE)
			return 1;

		// Full and Over Swing's hidden skills are at theirs.
		if (int32_t owner = aran_combo::swing_owner(id))
			id = owner;

		auto iter = skillentries.find(id);

		if (iter == skillentries.end())
			return 0;

		return iter->second.level;
	}

	int32_t SkillBook::get_masterlevel(int32_t id) const
	{
		auto iter = skillentries.find(id);

		if (iter == skillentries.end())
			return 0;

		return iter->second.masterlevel;
	}

	int64_t SkillBook::get_expiration(int32_t id) const
	{
		auto iter = skillentries.find(id);

		if (iter == skillentries.end())
			return 0;

		return iter->second.expiration;
	}

	std::map<int32_t, int32_t> SkillBook::collect_passives() const
	{
		std::map<int32_t, int32_t> passives;

		for (auto &iter : skillentries)
			if (SkillData::get(iter.first).is_passive())
				passives.emplace(iter.first, iter.second.level);

		return passives;
	}

	std::map<int32_t, int32_t> SkillBook::collect_levels() const
	{
		std::map<int32_t, int32_t> levels;

		for (auto &iter : skillentries)
			levels.emplace(iter.first, iter.second.level);

		return levels;
	}

	std::unordered_map<int32_t, int32_t> SkillBook::collect_required(int32_t id) const
	{
		auto iter = skillentries.find(id);

		if (iter == skillentries.end())
			return {};

		return SkillData::get(iter->first).get_reqskills();
	}
}
