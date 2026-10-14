#pragma once

#include "QuestObjectives.h"

#include <algorithm>
#include <cstdint>
#include <map>
#include <vector>

namespace ms
{
	// Spots started quests whose last objective was just met, for v83's
	// "Quest Complete!" notice. A quest is judged from when it is first seen,
	// so one already met when started or on login doesn't count.
	class QuestCompletion
	{
	public:
		// `started` holds each started quest's objectives. Returns the quests
		// that weren't all met last time and are now.
		std::vector<int16_t> update(const std::map<int16_t, std::vector<QuestObjective>>& started)
		{
			std::vector<int16_t> completed;
			std::map<int16_t, bool> now;

			for (const auto& [id, objectives] : started)
			{
				bool met = !objectives.empty() &&
					std::all_of(objectives.begin(), objectives.end(), [](const QuestObjective& o) { return o.met(); });
				auto before = this->met.find(id);

				if (met && before != this->met.end() && !before->second)
					completed.push_back(id);

				now[id] = met;
			}

			this->met = std::move(now);

			return completed;
		}

	private:
		std::map<int16_t, bool> met;
	};
}
