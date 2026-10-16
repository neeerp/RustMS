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
#include "CurseData.h"

#include "../Util/CurseWords.h"

#include <algorithm>
#include <utility>

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	namespace
	{
		// The words in the order of their numbers, which skip a few.
		const std::vector<std::string>& words()
		{
			static const std::vector<std::string> loaded = []()
			{
				std::vector<std::pair<int, std::string>> numbered;

				for (nl::node word : nl::nx::etc["Curse.img"])
					numbered.emplace_back(std::stoi(word.name()), word.get_string());

				std::sort(numbered.begin(), numbered.end());

				std::vector<std::string> result;

				for (auto& word : numbered)
					result.push_back(std::move(word.second));

				return result;
			}();

			return loaded;
		}
	}

	std::string CurseData::filter(const std::string& message)
	{
		return CurseWords::filter(message, words());
	}
}
