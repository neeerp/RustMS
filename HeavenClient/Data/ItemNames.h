#pragma once

#include "../Character/Pets.h"

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// Every item's name as the item searches match it (pets::search_key), in
	// String's order: equipment, use, set-up, cash, pets and etc.
	// (FUN_005d8796). The pet's item search and the Owl of Minerva's go
	// through it.
	inline const std::vector<std::pair<std::string, int32_t>> &item_search_names()
	{
		static std::vector<std::pair<std::string, int32_t>> names;

		if (!names.empty())
			return names;

		auto add = [&](nl::node items)
		{
			for (nl::node item : items)
				names.emplace_back(pets::search_key(item["name"].get_string()), std::stoi(item.name()));
		};

		for (nl::node category : nl::nx::string["Eqp.img"]["Eqp"])
			add(category);

		add(nl::nx::string["Consume.img"]);
		add(nl::nx::string["Ins.img"]);
		add(nl::nx::string["Cash.img"]);
		add(nl::nx::string["Pet.img"]);
		add(nl::nx::string["Etc.img"]["Etc"]);

		return names;
	}
}
