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

		bool is_started(int16_t);

		int16_t get_last_started();
		void clear();
		bool is_completed(int16_t id) const;
		const std::map<int16_t, std::string>& started_quests() const;
		const std::map<int16_t, int64_t>& completed_quests() const;
		// `count` gives how many of an item the player holds.
		QuestMarker npc_marker(int32_t npc_id, int16_t job, int16_t level,
			const std::function<int32_t(int32_t)>& count) const;
		uint64_t revision() const;
		int16_t state(int16_t id) const;
		std::vector<int16_t> available_quests(int16_t job, int16_t level, bool all_levels = false) const;

	private:
		std::map<int16_t, std::string> started;
		std::map<int16_t, std::pair<int16_t, std::string>> in_progress;
		std::map<int16_t, int64_t> completed;
		uint64_t marker_revision = 0;
	};
}
