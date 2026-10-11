#pragma once

#include <algorithm>
#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// Values match UIWindow2.img/QuestIcon animation IDs.
	enum class QuestMarker : int8_t { NONE = -1, AVAILABLE = 0, IN_PROGRESS = 1, READY = 2 };

	struct QuestMarkerRule
	{
		int16_t quest_id;
		int32_t start_npc;
		int32_t finish_npc;
		std::vector<int16_t> jobs;
		std::vector<std::pair<int16_t, int16_t>> prerequisites;
		bool completion_ready;
		int16_t max_start_level = 32767;
	};

	template<class Status>
	bool quest_available(const QuestMarkerRule& rule, int16_t job, int16_t level,
		Status status, bool all_levels = false)
	{
		return status(rule.quest_id) == 0 &&
			(rule.jobs.empty() || std::find(rule.jobs.begin(), rule.jobs.end(), job) != rule.jobs.end()) &&
			(all_levels || level <= rule.max_start_level) &&
			std::all_of(rule.prerequisites.begin(), rule.prerequisites.end(),
				[&](const auto& requirement) { return status(requirement.first) == requirement.second; });
	}

	// Keep selection independent of graphics/assets so chain transitions and
	// marker priority can be tested without launching the client.
	template<class Status>
	QuestMarker select_quest_marker(int32_t npc_id, int16_t job,
		const std::vector<QuestMarkerRule>& rules, Status status, int16_t level = 1)
	{
		QuestMarker result = QuestMarker::NONE;
		for (const auto& rule : rules)
		{
			if (!rule.jobs.empty() && std::find(rule.jobs.begin(), rule.jobs.end(), job) == rule.jobs.end()) continue;
			bool eligible = std::all_of(rule.prerequisites.begin(), rule.prerequisites.end(),
				[&](const auto& requirement) { return status(requirement.first) == requirement.second; });
			if (!eligible) continue;
			QuestMarker candidate = QuestMarker::NONE;
			if (status(rule.quest_id) == 0 && rule.start_npc == npc_id && level <= rule.max_start_level) candidate = QuestMarker::AVAILABLE;
			else if (status(rule.quest_id) == 1 && rule.start_npc == npc_id && rule.finish_npc != npc_id)
				candidate = QuestMarker::IN_PROGRESS;
			else if (status(rule.quest_id) == 1 && rule.finish_npc == npc_id)
				candidate = rule.completion_ready ? QuestMarker::READY : QuestMarker::IN_PROGRESS;
			if (static_cast<int>(candidate) > static_cast<int>(result)) result = candidate;
		}
		return result;
	}
}
