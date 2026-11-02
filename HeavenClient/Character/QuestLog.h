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
#include "QuestAlarm.h"
#include "QuestMarker.h"

namespace ms
{
	class Inventory;

	// Class that stores information on the quest log of an individual character
	class QuestLog
	{
	public:
		void add_started(int16_t, const std::string &quest_data);

		void add_in_progress(int16_t, int16_t, const std::string &quest_data);

		void add_completed(int16_t, int64_t);

		// Forget a quest, e.g. after it was forfeited.
		void remove(int16_t);

		// Quests shown in the Quest Helper. Those added by hand come before
		// those added automatically.
		static constexpr size_t MAX_TRACKED = QuestAlarm::MAX;
		const std::vector<int16_t>& tracked() const;
		bool is_tracked(int16_t id) const;
		// Add a quest by hand; `false` if the helper is full or the quest is
		// not started.
		bool track(int16_t id);
		void untrack(int16_t id);
		// Whether quests that make progress are added automatically (AUTO).
		bool auto_tracks() const;
		void set_auto_track(bool on);
		// Take this character's saved list and AUTO, after the quests are
		// loaded, and save changes under `key` from then on.
		void restore_tracking(const std::string& key);
		// A started quest's record changed (0xa20f4c, state 1): keep a tracked
		// one listed, or add it if it shows progress but isn't done (0xa212af).
		void record_updated(int16_t id, const Inventory& inventory);
		// The counts of the items started quests ask for, before an inventory
		// change, for items_changed.
		std::map<int32_t, int16_t> wanted_items(const Inventory& inventory) const;
		// Gaining a wanted item keeps a tracked quest listed or adds it
		// (0xa1f387).
		void items_changed(const std::map<int32_t, int16_t>& before, const Inventory& inventory);
		// Drop the quests added automatically that went ten minutes without
		// progress and aren't done (0x887b5c).
		void expire_tracking(const Inventory& inventory);

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
		// How many party quests' records (1200-1399) are at `rank`, one per
		// quest (0x724722).
		int32_t rank_count(const std::string& rank) const;
		// How many quests are completed as v83 counts them for questComplete
		// (counts_as_completed).
		int32_t completed_count() const;
		int16_t state(int16_t id) const;
		std::vector<int16_t> available_quests(int16_t job, int16_t level,
			const std::function<bool(const QuestInfo&)>& holds, bool all_levels = false) const;

	private:
		std::map<int16_t, std::string> started;
		std::map<int16_t, std::pair<int16_t, std::string>> in_progress;
		std::map<int16_t, int64_t> completed;
		std::map<int16_t, std::string> records_ex;
		// v83's automatic add (0x88599c with bAuto).
		bool offer(int16_t id, const Inventory& inventory, bool unless_done);
		void touch(int16_t id);
		bool objectives_met(int16_t id, const Inventory& inventory) const;
		void save_tracking() const;

		std::vector<int16_t> tracking;
		bool auto_track = true;
		// When each quest added automatically last made progress (+0x640).
		std::map<int16_t, int64_t> auto_since;
		// Quests the player took out of the helper; not re-added automatically.
		std::vector<int16_t> dismissed;
		// The character's key for the saved list; empty while loading.
		std::string tracking_key;
		uint64_t marker_revision = 0;
	};
}
