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
#include "Pet.h"

namespace ms
{
	Pet::Pet(int32_t item_id, int64_t expiration, const std::string &petname, uint8_t level, uint16_t closeness,
			 uint8_t fullness, int64_t date_dead, int32_t remain_life) : item_id(item_id), expiration(expiration),
								 petname(petname), petlevel(level), closeness(closeness), fullness(fullness),
								 date_dead(date_dead), remain_life(remain_life)
	{}

	int32_t Pet::get_item_id() const
	{
		return item_id;
	}

	const std::string &Pet::get_name() const
	{
		return petname;
	}

	uint8_t Pet::get_level() const
	{
		return petlevel;
	}

	uint16_t Pet::get_closeness() const
	{
		return closeness;
	}

	uint8_t Pet::get_fullness() const
	{
		return fullness;
	}

	int64_t Pet::get_date_dead() const
	{
		return date_dead;
	}

	int32_t Pet::get_remain_life() const
	{
		return remain_life;
	}
}
