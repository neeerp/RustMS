#pragma once

#include "QuestInfo.h"

#include <algorithm>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// Values match UIWindow2.img/QuestIcon animation IDs.
	enum class QuestMarker : int8_t { NONE = -1, AVAILABLE = 0, IN_PROGRESS = 1, READY = 2 };

	// Kill counts in a started quest's progress string: three digits per
	// required mob ("003012" for 3 and 12).
	inline std::vector<int16_t> decode_kills(const std::string& progress)
	{
		std::vector<int16_t> kills;
		for (size_t i = 0; i + 3 <= progress.size(); i += 3)
		{
			int16_t count = 0;
			for (size_t j = i; j < i + 3; j++)
				count = count * 10 + (progress[j] >= '0' && progress[j] <= '9' ? progress[j] - '0' : 0);
			kills.push_back(count);
		}
		return kills;
	}

	// Whether a started quest's progress string covers its required kills.
	inline bool end_mobs_met(const QuestInfo& quest, const std::string& progress)
	{
		std::vector<int16_t> kills = decode_kills(progress);
		for (size_t i = 0; i < quest.end_mobs.size(); i++)
			if ((i < kills.size() ? kills[i] : 0) < quest.end_mobs[i].second)
				return false;
		return true;
	}

	// Whether the player's items satisfy a quest's completion requirements.
	// `count` gives how many of an item the player holds.
	template<class Count>
	bool end_items_met(const QuestInfo& quest, Count count)
	{
		return std::all_of(quest.end_items.begin(), quest.end_items.end(), [&](const auto& requirement)
		{
			int32_t held = count(requirement.first);
			return requirement.second > 0 ? held >= requirement.second : held == 0;
		});
	}

	template<class Status>
	bool quest_available(const QuestInfo& quest, int16_t job, int16_t level,
		Status status, bool all_levels = false)
	{
		return status(quest.id) == 0 &&
			(quest.jobs.empty() || std::find(quest.jobs.begin(), quest.jobs.end(), job) != quest.jobs.end()) &&
			(all_levels || (level >= quest.min_level && level <= quest.max_level)) &&
			std::all_of(quest.prerequisites.begin(), quest.prerequisites.end(),
				[&](const auto& requirement) { return status(requirement.first) == requirement.second; });
	}

	// The icon over an NPC, as v83 chooses it (FUN_006d1779). The NPC's quests
	// are those it starts or finishes, less party quests (1200-1399). A started
	// quest it can finish now (kills and items in) gives the closed book;
	// otherwise a quest it can start gives the bulb, and a started quest the
	// open book. Kept free of graphics so the choice can be tested.
	// `progress` gives a started quest's progress string (its kill counts).
	template<class Status, class Count, class Progress>
	QuestMarker select_quest_marker(int32_t npc_id, int16_t job,
		const std::vector<QuestInfo>& quests, Status status, int16_t level, Count count,
		Progress progress)
	{
		bool available = false, in_progress = false;

		for (const auto& quest : quests)
		{
			if (quest.start_npc != npc_id && quest.end_npc != npc_id)
				continue;

			if (quest.id >= 1200 && quest.id < 1400)
				continue;

			if (status(quest.id) == 1)
			{
				if ((quest.end_npc == 0 || quest.end_npc == npc_id) &&
					end_items_met(quest, count) && end_mobs_met(quest, progress(quest.id)))
					return QuestMarker::READY;

				in_progress = true;
			}
			else if ((quest.start_npc == 0 || quest.start_npc == npc_id) &&
				quest_available(quest, job, level, status))
			{
				available = true;
			}
		}

		if (available)
			return QuestMarker::AVAILABLE;

		return in_progress ? QuestMarker::IN_PROGRESS : QuestMarker::NONE;
	}
}
