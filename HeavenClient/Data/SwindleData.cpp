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
#include "SwindleData.h"

#include "../Util/Randomizer.h"
#include "../Util/SwindleWords.h"

#include <chrono>

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	namespace
	{
		std::vector<std::string> strings(nl::node list)
		{
			std::vector<std::string> result;

			for (size_t i = 0; i < list.size(); i++)
				result.push_back(list[std::to_string(i)].get_string());

			return result;
		}

		const std::vector<SwindleWords::Group>& groups()
		{
			static const std::vector<SwindleWords::Group> loaded = []()
			{
				std::vector<SwindleWords::Group> result;
				nl::node src = nl::nx::etc["Swindle.img"];

				for (size_t i = 0; i < src.size(); i++)
				{
					nl::node entry = src[std::to_string(i)];
					result.push_back({ strings(entry["word"]), strings(entry["warn"]) });
				}

				return result;
			}();

			return loaded;
		}

		constexpr auto COOLDOWN = std::chrono::milliseconds(10'000);
	}

	std::optional<std::string> SwindleData::warning(const std::string& message)
	{
		static std::optional<std::chrono::steady_clock::time_point> last_warning;

		auto group = SwindleWords::match(groups(), message);

		if (!group)
			return std::nullopt;

		auto now = std::chrono::steady_clock::now();

		if (last_warning && now - *last_warning < COOLDOWN)
			return std::nullopt;

		const auto& warnings = groups()[*group].warnings;

		if (warnings.empty())
			return std::nullopt;

		last_warning = now;

		return warnings[Randomizer().next_int(warnings.size())];
	}
}
