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
#include "SkillAction.h"

#include "AranCombo.h"

#include "../../Util/Misc.h"
#include "../../Util/Randomizer.h"

namespace ms
{
	std::string RegularAction::apply(Char &target, Attack::Type atype) const
	{
		Weapon::Type weapontype = target.get_weapontype();
		bool degenerate;

		switch (weapontype)
		{
			case Weapon::BOW:
			case Weapon::CROSSBOW:
			case Weapon::CLAW:
			case Weapon::GUN:
				degenerate = atype != Attack::RANGED;
				break;
			default:
				degenerate = false;
				break;
		}

		if (auto swing = aran_combo::polearm_swing(target.get_job(), weapontype, Randomizer().next_int(3)))
		{
			target.attack(degenerate, swing->stance);
			// As a std::string: a bare const char* would pick attack(bool).
			target.attack(std::string(swing->action));

			return "";
		}

		target.attack(degenerate);

		return "";
	}

	RandomAction::RandomAction(nl::node src)
	{
		for (size_t i = 0; ; i++)
		{
			nl::node sub = src["action"][std::to_string(i)];

			if (sub.data_type() != nl::node::type::string)
				break;

			actions.push_back(std::string(sub));
		}
	}

	std::string RandomAction::apply(Char &target, Attack::Type) const
	{
		Randomizer randomizer;
		const std::string &action = actions[randomizer.next_int(actions.size())];
		target.attack(action);

		return action;
	}

	ByLevelAction::ByLevelAction(nl::node src, int32_t id)
	{
		for (auto sub : src["level"])
		{
			int32_t level = string_conversion::or_zero<int32_t>(sub.name());
			actions[level] = std::string(sub["action"]);
		}

		skillid = id;
	}

	std::string ByLevelAction::apply(Char &target, Attack::Type) const
	{
		auto iter = actions.find(target.get_skilllevel(skillid));

		if (iter == actions.end())
			return "";

		target.attack(iter->second);

		return iter->second;
	}
}