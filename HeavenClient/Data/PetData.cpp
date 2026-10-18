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
#include "PetData.h"

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	namespace
	{
		// An answer: `act` and lines 0, 1, ... that are PetDialog keys.
		pets::Reaction reaction(nl::node src, nl::node dialog)
		{
			pets::Reaction reaction;
			reaction.act = src["act"].get_string();

			for (int i = 0; src[std::to_string(i)]; i++)
				reaction.lines.push_back(dialog[src[std::to_string(i)].get_string()].get_string());

			return reaction;
		}

		std::vector<pets::Reaction> reactions(nl::node src, nl::node dialog)
		{
			std::vector<pets::Reaction> list;

			for (int i = 0; src[std::to_string(i)]; i++)
				list.push_back(reaction(src[std::to_string(i)], dialog));

			return list;
		}
	}

	PetData::PetData(int32_t itemid)
	{
		std::string strid = std::to_string(itemid);
		nl::node src = nl::nx::item["Pet"][strid + ".img"];
		nl::node dialog = nl::nx::string["PetDialog.img"][strid];

		kind = nl::nx::string["Pet.img"][strid]["name"].get_string();
		balloon = static_cast<int8_t>(src["info"]["chatBalloon"].get_integer());

		nl::node interact = src["interact"];

		for (int i = 0; interact[std::to_string(i)]; i++)
		{
			nl::node command = interact[std::to_string(i)];

			commands.push_back({ static_cast<int16_t>(command["l0"].get_integer()),
								 static_cast<int16_t>(command["l1"].get_integer()),
								 pets::split_words(dialog[command["command"].get_string()].get_string()),
								 reactions(command["success"], dialog), reactions(command["fail"], dialog) });
		}

		idle = reactions(src["randAction"], dialog);

		for (nl::node child : src)
			if (!pets::reserved(child.name()))
				customs.push_back(child.name());
	}

	const std::string &PetData::get_kind() const
	{
		return kind;
	}

	int8_t PetData::get_balloon() const
	{
		return balloon;
	}

	const std::vector<pets::Command> &PetData::get_commands() const
	{
		return commands;
	}

	const std::vector<pets::Reaction> &PetData::get_idle() const
	{
		return idle;
	}

	const std::vector<std::string> &PetData::get_customs() const
	{
		return customs;
	}
}
