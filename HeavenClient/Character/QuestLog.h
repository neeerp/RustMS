//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include <string>

#include <cstdint>

#include <functional>
#include <map>
#include <vector>
#include "QuestMarker.h"

namespace ms
{
	// Class that stores information on the quest log of an individual character
	class QuestLog
	{
	public:
		void add_started(int16_t, const std::string &quest_data);

		void add_in_progress(int16_t, int16_t, const std::string &quest_data);

		void add_completed(int16_t, int64_t);

		// Forget a quest, e.g. after it was forfeited.
		void remove(int16_t);

		// Quests shown in the Quest Helper, oldest first. Newly started quests
		// with something to hunt or collect are tracked automatically.
		static constexpr size_t MAX_TRACKED = 5;
		const std::vector<int16_t>& tracked() const;
		bool is_tracked(int16_t id) const;
		// `false` if the helper is full or the quest is not started.
		bool track(int16_t id);
		void untrack(int16_t id);
		// Whether newly started quests are tracked (the helper's AUTO).
		bool auto_tracks() const;
		void set_auto_track(bool on);

		bool is_started(int16_t);

		int16_t get_last_started();
		void clear();
		bool is_completed(int16_t id) const;
		const std::map<int16_t, std::string>& started_quests() const;
		const std::map<int16_t, int64_t>& completed_quests() const;
		// `count` gives how many of an item the player holds, and `holds`
		// whether they meet a quest's start_holdings_met.
		QuestMarker npc_marker(int32_t npc_id, int16_t job, int16_t level,
			const std::function<int32_t(int32_t)>& count,
			const std::function<bool(const QuestInfo&)>& holds) const;
		uint64_t revision() const;
		// The records kept beside the quest log ("quest record ex"), each
		// "k=v;k=v" as the server sends it whole, which the Quest Log's #j
		// tokens read (the party quests' tries, clears and ranks).
		void set_record_ex(int16_t id, const std::string& raw);
		// The value under `key` in quest `id`'s record, or "".
		std::string record_ex(int16_t id, const std::string& key) const;
		int16_t state(int16_t id) const;
		std::vector<int16_t> available_quests(int16_t job, int16_t level,
			const std::function<bool(const QuestInfo&)>& holds, bool all_levels = false) const;

	private:
		std::map<int16_t, std::string> started;
		std::map<int16_t, std::pair<int16_t, std::string>> in_progress;
		std::map<int16_t, int64_t> completed;
		std::map<int16_t, std::string> records_ex;
		std::vector<int16_t> tracking;
		bool auto_track = true;
		// Quests the player took out of the helper; not re-added automatically.
		std::vector<int16_t> dismissed;
		uint64_t marker_revision = 0;
	};
}
