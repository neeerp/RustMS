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
#include "SkillBullet.h"
#include "SoulArrow.h"

#include "../Data/BulletData.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Animation RegularBullet::get(const Char &, int32_t bulletid) const
	{
		// A Soul Arrow shot flies that skill's ball rather than an arrow.
		if (soul_arrow::is_ball(bulletid))
			return nl::nx::skill[string_format::skill_img(bulletid)]["skill"][std::to_string(bulletid)]["ball"];

		return BulletData::get(bulletid).get_animation();
	}

	SingleBullet::SingleBullet(nl::node src)
	{
		ball = src["ball"];

		// Chain Lightning's ball is three kinds of ball, each animated.
		for (size_t kind = 0; src["ball"][std::to_string(kind)]["0"].data_type() == nl::node::type::bitmap; kind++)
			kinds.emplace_back(src["ball"][std::to_string(kind)]);
	}

	Animation SingleBullet::get(const Char &, int32_t) const
	{
		return ball.animation;
	}

	Animation SingleBullet::get_kind(size_t kind) const
	{
		return kind < kinds.size() ? kinds[kind].animation : Animation();
	}

	BySkillLevelBullet::BySkillLevelBullet(nl::node src, int32_t id)
	{
		skillid = id;

		for (auto sub : src["level"])
		{
			auto level = string_conversion::or_zero<int32_t>(sub.name());
			bullets[level] = sub["ball"];
		}
	}

	Animation BySkillLevelBullet::get(const Char &user, int32_t) const
	{
		int32_t level = user.get_skilllevel(skillid);
		auto iter = bullets.find(level);

		if (iter != bullets.end())
			return iter->second.animation;
		else
			return {};
	}
}