#include "PartyQuests.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// A GM may go to any.
		bool job_allowed(const PartyQuest& quest, int16_t job)
		{
			return quest.jobs.empty() || job % 1000 / 100 == 9 ||
				std::find(quest.jobs.begin(), quest.jobs.end(), job) != quest.jobs.end();
		}
	}

	std::vector<const PartyQuest*> party_quest_cards(const std::vector<PartyQuest>& quests, int16_t job, int16_t level)
	{
		std::vector<const PartyQuest*> cards;

		for (const PartyQuest& quest : quests)
			if (quest.min_level == level && job_allowed(quest, job))
				cards.push_back(&quest);

		return cards;
	}

	bool party_quest_listed(const PartyQuest& quest, int16_t job, int16_t level, bool all_levels)
	{
		bool in_range = all_levels || (level >= quest.min_level - 5 && level <= quest.max_level);

		return in_range && job_allowed(quest, job);
	}
}
