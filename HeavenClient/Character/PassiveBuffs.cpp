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
#include "PassiveBuffs.h"

#include "FairyBlessing.h"
#include "SecondaryStat.h"
#include "SkillId.h"

#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	bool ConditionlessBuff::is_applicable(CharStats &, nl::node) const
	{
		return true;
	}

	void AngelBlessingBuff::apply_to(CharStats &stats, nl::node level) const
	{
		stats.add_value(EquipStat::Id::WATK, level["x"]);
		stats.add_value(EquipStat::Id::MAGIC, level["y"]);
		stats.add_value(EquipStat::Id::ACC, level["z"]);
		stats.add_value(EquipStat::Id::AVOID, level["z"]);
	}

	void AchillesBuff::apply_to(CharStats &stats, nl::node level) const
	{
		// Achilles leaves x per mille of a hit.
		float reducedamage = 1.0f - static_cast<float>(level["x"]) / 1000;
		stats.set_reducedamage(reducedamage);
	}

	void NimbleBuff::apply_to(CharStats &stats, nl::node level) const
	{
		stats.add_value(EquipStat::Id::ACC, level["x"]);
		stats.add_value(EquipStat::Id::AVOID, level["y"]);
	}

	void ThrustBuff::apply_to(CharStats &stats, nl::node level) const
	{
		stats.add_value(EquipStat::Id::SPEED, level["speed"]);
	}

	PassiveBuffs::PassiveBuffs()
	{
		// Beginner
		buffs[SkillId::Id::ANGEL_BLESSING] = std::make_unique<AngelBlessingBuff>();
		// The Noblesse's and the Legend's (SetFrom, 0x7801bb).
		buffs[SkillId::Id::NOBLESSE_BLESSING] = std::make_unique<AngelBlessingBuff>();
		buffs[SkillId::Id::LEGEND_BLESSING] = std::make_unique<AngelBlessingBuff>();

		// Hero
		buffs[SkillId::Id::ACHILLES_HERO] = std::make_unique<AchillesBuff>();

		// Paladin
		buffs[SkillId::Id::ACHILLES_PALADIN] = std::make_unique<AchillesBuff>();

		// Dark Knight
		buffs[SkillId::Id::ACHILLES_DK] = std::make_unique<AchillesBuff>();

		// Aran (0x959321)
		buffs[SkillId::Id::ACHILLES_ARAN] = std::make_unique<AchillesBuff>();

		for (int32_t id : secondary_stat::NIMBLE_SKILLS)
			buffs[id] = std::make_unique<NimbleBuff>();

		for (int32_t id : secondary_stat::THRUSTS)
			buffs[id] = std::make_unique<ThrustBuff>();
	}

	void PassiveBuffs::apply_buff(CharStats &stats, int32_t skill_id, int32_t skill_level) const
	{
		auto iter = buffs.find(skill_id);

		if (iter == buffs.end())
			return;

		// Blessing of the Fairy is the job tree's beginner skill, which
		// can_use refuses past the first advancement.
		bool wrong_job = !fairy_blessing::is_blessing(skill_id) && !stats.get_job().can_use(skill_id);

		if (wrong_job)
			return;

		std::string strid;

		if (skill_id < 10000000)
			strid = string_format::extend_id(skill_id, 7);
		else
			strid = std::to_string(skill_id);

		nl::node src = nl::nx::skill[string_format::skill_img(skill_id)]["skill"][strid]["level"][skill_level];

		const PassiveBuff *buff = iter->second.get();

		if (buff && buff->is_applicable(stats, src))
			buff->apply_to(stats, src);
	}
}