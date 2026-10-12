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

	// Keep selection independent of graphics/assets so chain transitions and
	// marker priority can be tested without launching the client.
	// `progress` gives a started quest's progress string (its kill counts).
	template<class Status, class Count, class Progress>
	QuestMarker select_quest_marker(int32_t npc_id, int16_t job,
		const std::vector<QuestInfo>& quests, Status status, int16_t level, Count count,
		Progress progress)
	{
		QuestMarker result = QuestMarker::NONE;
		for (const auto& quest : quests)
		{
			if (!quest.jobs.empty() && std::find(quest.jobs.begin(), quest.jobs.end(), job) == quest.jobs.end()) continue;
			bool eligible = std::all_of(quest.prerequisites.begin(), quest.prerequisites.end(),
				[&](const auto& requirement) { return status(requirement.first) == requirement.second; });
			if (!eligible) continue;
			QuestMarker candidate = QuestMarker::NONE;
			if (status(quest.id) == 0 && quest.start_npc == npc_id &&
				level >= quest.min_level && level <= quest.max_level) candidate = QuestMarker::AVAILABLE;
			else if (status(quest.id) == 1 && quest.start_npc == npc_id && quest.end_npc != npc_id)
				candidate = QuestMarker::IN_PROGRESS;
			else if (status(quest.id) == 1 && quest.end_npc == npc_id)
				candidate = end_items_met(quest, count) && end_mobs_met(quest, progress(quest.id)) ?
					QuestMarker::READY : QuestMarker::IN_PROGRESS;
			if (static_cast<int>(candidate) > static_cast<int>(result)) result = candidate;
		}
		return result;
	}
}
