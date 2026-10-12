#pragma once

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// A quest from Quest.nx that the server supports.
	struct QuestInfo
	{
		int16_t id;
		std::string name;
		int32_t start_npc;
		int32_t end_npc;
		std::vector<int16_t> jobs;
		int16_t min_level = 0;
		int16_t max_level = 32767;
		// (quest id, required state) pairs.
		std::vector<std::pair<int16_t, int16_t>> prerequisites;
		// Items needed to complete: (item id, count); a count of 0 means none
		// may be left.
		std::vector<std::pair<int32_t, int32_t>> end_items;
		// Mobs to kill before completing: (mob id, count).
		std::vector<std::pair<int32_t, int16_t>> end_mobs;
		// The server runs these steps as scripts (QUEST_ACTION 4/5).
		bool start_script = false;
		bool end_script = false;
		int32_t exp = 0;
		int16_t next_quest = 0;
	};
}
