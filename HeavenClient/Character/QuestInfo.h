#pragma once

#include "QuestRewards.h"

#include <cstdint>
#include <string>
#include <tuple>
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
		// The closeness range that pet must have (pettamenessmin and
		// pettamenessmax, 0 and 30000 when unset), read only with a `pet`
		// list.
		std::pair<int32_t, int32_t> pet_closeness{ 0, 30000 };
		std::pair<int32_t, int32_t> end_pet_closeness{ 0, 30000 };
		// An item whose effect, as if used, must be on the player (buff) or
		// must not be (exceptbuff), to start and to complete; 0 for none.
		int32_t buff = 0;
		int32_t except_buff = 0;
		int32_t end_buff = 0;
		int32_t end_except_buff = 0;
		// The fewest and most different cards the Monster Book may hold to
		// complete (Check/1/mbmin, mbmax); -1 for no bound.
		int32_t end_book_min = -1;
		int32_t end_book_max = -1;
		// Cards whose count in the book must lie within bounds to complete
		// (Check/1/mbcard), as (card, min, max) with -1 for no bound.
		std::vector<std::tuple<int32_t, int32_t, int32_t>> end_cards;
		// Least fame to start (Check/0/pop); 0 means any.
		int16_t min_fame = 0;
		// Not offered before this FILETIME (Check/0/start, quest_date); 0
		// means any time.
		int64_t start_time = 0;
		// Nor after this one (Check/0/end); most event quests ended in
		// 2005-2010 and are never offered.
		int64_t end_time = INT64_MAX;
		// Minutes after completing before the quest can be started again
		// (Check/0/interval); -1 means it can't be repeated.
		int32_t repeat_interval = -1;
		// Without an interval, repeatable on a later day (Check/0/dayByDay).
		bool day_by_day = false;
		// The weekdays it can be started on (Check/0/dayOfWeek), a bit each
		// from Sunday (1) to Saturday (64); every day without the key.
		uint8_t weekdays = 0xFF;
		// Items needed to complete: (item id, count); a count of 0 means none
		// may be left.
		std::vector<std::pair<int32_t, int32_t>> end_items;
		// Quests in a state needed to complete this one, as above.
		std::vector<std::pair<int16_t, int16_t>> end_prerequisites;
		// Mesos to hold to complete (Check/1/endmeso).
		int32_t end_meso = 0;
		// The quest whose record (its data string) Check/1/infoex looks at
		// (Check/1/infoNumber, 0 for none), and the entries, (cond, value),
		// of which one must hold.
		int16_t end_info_record = 0;
		std::vector<std::pair<int32_t, std::string>> end_info;
		// The same for starting it (Check/0), kept only for a record that is
		// no quest, such as the Mushroom Kingdom's opening scene's 2311.
		int16_t start_info_record = 0;
		std::vector<std::pair<int32_t, std::string>> start_info;
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
