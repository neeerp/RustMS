#pragma once

#include <cstdint>
#include <set>
#include <string>
#include <vector>

namespace ms
{
	namespace quest_result
	{
		// An item of a quest step, from Act.img/<id>/<0|1>/item.
		struct ActItem
		{
			int32_t id;
			int32_t count;
			// A mask of job families, 1 << (job / 100); -1 when absent.
			int32_t job;
			// 0 male, 1 female, 2 either.
			int32_t gender;
		};

		// What v83 says when a quest step's items don't fit (quest result
		// 10, FUN_00959a73): the inventories of the items the character
		// would get, then the mesos too if the step takes some. v83 joins
		// the inventories with " or" and no space after it.
		inline std::string inventory_full(const std::vector<ActItem>& items, int32_t money, int16_t job, bool female)
		{
			std::set<int32_t> seen;
			std::string inventories;

			for (const ActItem& item : items)
			{
				if (item.count <= 0)
					continue;

				if (!(item.job & (1 << (job / 100))) && job / 100 != 9)
					continue;

				if (item.gender != 2 && item.gender != (female ? 1 : 0))
					continue;

				int32_t type = item.id / 1000000;

				if (!seen.insert(type).second)
					continue;

				if (!inventories.empty())
					inventories += " or";

				switch (type)
				{
				case 1:
					inventories += "Eqp";
					break;
				case 2:
					inventories += "Use";
					break;
				case 3:
					inventories += "Setup";
					break;
				case 4:
					inventories += "Etc";
					break;
				}
			}

			std::string text = inventories + " item inventory is full.";

			if (money < 0)
				text = "Either you don't have enough Mesos or " + text;

			return text;
		}
	}
}
