#pragma once

#include "QuestRewards.h"

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
		// Items to hold to start (Check/0/item): (item id, count), where a
		// positive count is a minimum, 0 means none, and a negative count a
		// maximum.
		std::vector<std::pair<int32_t, int32_t>> held_items;
		// Skills to have (true, from `acquire`) or lack (false) to start.
		std::vector<std::pair<int32_t, bool>> skills;
		// Items that must all be worn (equipAllNeed), and of which one must
		// be worn (equipSelectNeed), to start.
		std::vector<int32_t> worn_all;
		std::vector<int32_t> worn_any;
		// Pet items of which the first pet out must be one (Check/0/pet and
		// Check/1/pet); empty means no pet is needed.
		std::vector<int32_t> pets;
		std::vector<int32_t> end_pets;
		// Least fame to start (Check/0/pop); 0 means any.
		int16_t min_fame = 0;
		// Not offered before this FILETIME (Check/0/start, quest_date); 0
		// means any time.
		int64_t start_time = 0;
		// Minutes after completing before the quest can be started again
		// (Check/0/interval); -1 means it can't be repeated.
		int32_t repeat_interval = -1;
		// Items needed to complete: (item id, count); a count of 0 means none
		// may be left.
		std::vector<std::pair<int32_t, int32_t>> end_items;
		// Quests in a state needed to complete this one, as above.
		std::vector<std::pair<int16_t, int16_t>> end_prerequisites;
		// Mesos to hold to complete (Check/1/endmeso).
		int32_t end_meso = 0;
		// Items given when the quest starts: (item id, count).
		std::vector<std::pair<int32_t, int32_t>> start_items;
		// Mobs to kill before completing: (mob id, count).
		std::vector<std::pair<int32_t, int16_t>> end_mobs;
		// The server runs these steps as scripts (QUEST_ACTION 4/5).
		bool start_script = false;
		bool end_script = false;
		QuestRewards rewards;
		int16_t next_quest = 0;
		// QuestInfo.img/<id>/type, such as "[Job Adv.]": shown before the
		// name in an NPC's quest menu.
		std::string type;
		// QuestInfo.img flags for the quest alert, the bulb over the player: it offers autoStart quests and completes
		// autoComplete ones wherever the player is, and lists started autoPreComplete ones. An autoAccept quest opens
		// by itself instead.
		bool auto_start = false;
		bool auto_accept = false;
		bool auto_pre_complete = false;
		bool auto_complete = false;
		// When the alert checks an autoStart quest (Check/0): on every stat change, or on entering one of these maps.
		// The maps are also a start requirement: the player must be on one.
		bool normal_auto_start = false;
		std::vector<int32_t> field_enter;
	};
}
