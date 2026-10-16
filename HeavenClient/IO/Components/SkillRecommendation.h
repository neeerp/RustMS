#pragma once

#include <algorithm>
#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// The skill v83's skill book recommends raising (FUN_008abf71). A job's
	// Etc/RecommendSkill.img entries map SP spent in the job to a skill: the
	// last entry at or below the SP spent names it, and none does once the
	// SP spent passes the last entry. Returns 0 for no recommendation.
	inline int32_t recommended_skill(std::vector<std::pair<int32_t, int32_t>> table, int32_t spent)
	{
		std::sort(table.begin(), table.end());

		int32_t previous = 0;

		for (auto [at, skill] : table)
		{
			if (spent <= at)
				return spent == at ? skill : previous;

			previous = skill;
		}

		return 0;
	}
}
