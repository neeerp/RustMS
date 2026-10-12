#pragma once

#include "QuestMarker.h"

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// One thing a started quest still asks for: mobs to hunt or items to bring.
	struct QuestObjective
	{
		bool mob;
		int32_t id;
		int32_t have;
		int32_t need;

		bool met() const
		{
			return have >= need;
		}

		bool operator==(const QuestObjective& other) const
		{
			return mob == other.mob && id == other.id && have == other.have && need == other.need;
		}
	};

	// A started quest's objectives, mobs first, from its progress string and
	// the player's item counts. Items that must be used up (count 0) are not
	// listed.
	template<class Count>
	std::vector<QuestObjective> quest_objectives(const std::vector<std::pair<int32_t, int16_t>>& mobs,
		const std::vector<std::pair<int32_t, int32_t>>& items, const std::string& progress, Count count)
	{
		std::vector<QuestObjective> result;
		std::vector<int16_t> kills = decode_kills(progress);

		for (size_t i = 0; i < mobs.size(); i++)
		{
			int32_t have = i < kills.size() ? kills[i] : 0;
			result.push_back({ true, mobs[i].first, std::min<int32_t>(have, mobs[i].second), mobs[i].second });
		}

		for (const auto& [item_id, need] : items)
			if (need > 0)
				result.push_back({ false, item_id, std::min(count(item_id), need), need });

		return result;
	}
}
