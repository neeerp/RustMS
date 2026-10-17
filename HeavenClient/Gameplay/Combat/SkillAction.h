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
#include <vector>

#include <cstdint>

#include "Attack.h"

#include "../../Character/Char.h"

namespace ms
{
	class SkillAction
	{
	public:
		virtual ~SkillAction()
		{}

		// Plays the skill's body action, and returns it, or "" if it has
		// none of its own.
		virtual std::string apply(Char &target, Attack::Type atype) const = 0;
	};

	class NoAction : public SkillAction
	{
	public:
		std::string apply(Char &, Attack::Type) const override
		{
			return "";
		}
	};

	class RegularAction : public SkillAction
	{
	public:
		std::string apply(Char &target, Attack::Type atype) const override;
	};

	// v83 takes one of a skill's actions at random each time
	// (FUN_007660b6), so most skills, having one, always take it.
	class RandomAction : public SkillAction
	{
	public:
		RandomAction(nl::node src);

		std::string apply(Char &target, Attack::Type atype) const override;

	private:
		std::vector<std::string> actions;
	};

	class ByLevelAction : public SkillAction
	{
	public:
		ByLevelAction(nl::node src, int32_t skillid);

		std::string apply(Char &target, Attack::Type atype) const override;

	private:
		std::map<int32_t, std::string> actions;
		int32_t skillid;
	};
}