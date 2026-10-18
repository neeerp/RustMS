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
#include "TeleportRock.h"

#include "../Util/CharacterName.h"

#include <cctype>

namespace ms
{
	std::string TeleportRock::trim(const std::string &name)
	{
		const char *blanks = " \t\r\n";
		size_t first = name.find_first_not_of(blanks);

		if (first == std::string::npos)
			return "";

		return name.substr(first, name.find_last_not_of(blanks) - first + 1);
	}

	std::string TeleportRock::keep_problem(const std::vector<int32_t> &maps, int32_t map_id)
	{
		if (std::find(maps.begin(), maps.end(), EMPTY) == maps.end())
			return "Your teleport list is full.\\nPlease delete an entry before trying again.";

		if (std::find(maps.begin(), maps.end(), map_id) != maps.end())
			return "You have already entered this map.";

		if (!may_keep(map_id))
			return NOT_FOR_LIST;

		return "";
	}

	std::string TeleportRock::target_problem(const std::string &name, const std::string &own_name)
	{
		if (!character_name_valid(name))
			return "- The character name is invalid.";

		bool own = std::equal(name.begin(), name.end(), own_name.begin(), own_name.end(), [](char a, char b)
		{
			return std::tolower(static_cast<unsigned char>(a)) == std::tolower(static_cast<unsigned char>(b));
		});

		if (own)
			return "You cannot target yourself\\nfor the teleport.";

		return "";
	}

	std::string TeleportRock::refusal(int8_t result, const std::string &target)
	{
		switch (result)
		{
			case 5:
			case 8:
				return "You cannot go to that place.";
			case 6:
			case 7:
				return target + " is currently difficult to locate, so\\nthe teleport will not take place.";
			case 9:
				return SAME_MAP;
			case 10:
				return NOT_FOR_LIST;
			case 11:
				return "Users below level 7 are not allowed \\nto go out from Maple Island.";
			default:
				return "";
		}
	}

	TeleportRock::TeleportRock() : regular(slots(false), EMPTY), vip(slots(true), EMPTY)
	{}

	void TeleportRock::set_maps(bool is_vip, std::vector<int32_t> maps)
	{
		maps.resize(slots(is_vip), EMPTY);
		(is_vip ? vip : regular) = std::move(maps);
	}

	const std::vector<int32_t> &TeleportRock::get_maps(bool is_vip) const
	{
		return is_vip ? vip : regular;
	}

	void TeleportRock::set_target(const std::string &name)
	{
		target = name;
	}

	const std::string &TeleportRock::get_target() const
	{
		return target;
	}
}
