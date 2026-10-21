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
#include <cctype>
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

		std::string to_lower(std::string value)
		{
			std::transform(value.begin(), value.end(), value.begin(), [](unsigned char c)
			{
				return static_cast<char>(std::tolower(c));
			});

			return value;
		}
	}

	std::string CurseData::filter(const std::string& message)
	{
		return CurseWords::filter(message, words());
	}

	bool CurseData::in_name(const std::string& name)
	{
		std::string lower = to_lower(name);

		for (auto forbidden : nl::nx::etc["ForbiddenName.img"])
			if (lower.find(to_lower(forbidden)) != std::string::npos)
				return true;

		for (auto curse : nl::nx::etc["Curse.img"])
		{
			std::string word = curse;
			word = word.substr(0, word.find(','));
			word.erase(0, word.find_first_not_of(' '));
			word.erase(word.find_last_not_of(' ') + 1);

			if (!word.empty() && lower.find(to_lower(word)) != std::string::npos)
				return true;
		}

		return false;
	}
}
