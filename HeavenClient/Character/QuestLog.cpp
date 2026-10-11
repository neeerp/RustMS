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
#include "QuestLog.h"
#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	void QuestLog::add_started(int16_t qid, const std::string &qdata)
	{
		started[qid] = qdata;
		++marker_revision;
	}

	void QuestLog::add_in_progress(int16_t qid, int16_t qidl, const std::string &qdata)
	{
		in_progress[qid] = make_pair(qidl, qdata);
		++marker_revision;
	}

	void QuestLog::add_completed(int16_t qid, int64_t time)
	{
		started.erase(qid);
		in_progress.erase(qid);
		completed[qid] = time;
		++marker_revision;
	}

	void QuestLog::clear() { started.clear(); in_progress.clear(); completed.clear(); ++marker_revision; }
	bool QuestLog::is_completed(int16_t id) const { return completed.count(id) != 0; }
	const std::map<int16_t, std::string>& QuestLog::started_quests() const { return started; }
	const std::map<int16_t, int64_t>& QuestLog::completed_quests() const { return completed; }
	uint64_t QuestLog::revision() const { return marker_revision; }

	QuestMarker QuestLog::npc_marker(int32_t npc_id, int16_t job, int16_t level) const
	{
		static const auto rules = []()
		{
			std::vector<QuestMarkerRule> result;
			// Only advertise the quest chain implemented by the server. The NX
			// files also contain many quests whose objectives are not supported.
			for (int16_t id : {1009, 1010, 1011, 1012, 1013, 1014, 1015, 1031, 1032, 1033})
			{
				auto check = nl::nx::quest["Check.img"][std::to_string(id)];
				auto start = check["0"];
				auto finish = check["1"];
				if (start["npc"].data_type() != nl::node::type::integer ||
					finish["npc"].data_type() != nl::node::type::integer) continue;
				QuestMarkerRule rule { id, static_cast<int32_t>(start["npc"].get_integer()),
					static_cast<int32_t>(finish["npc"].get_integer()), {}, {}, true };
				for (auto allowed_job : start["job"]) rule.jobs.push_back(allowed_job.get_integer());
				if (start["lvmax"].data_type() == nl::node::type::integer)
					rule.max_start_level = start["lvmax"].get_integer();
				for (auto prerequisite : start["quest"])
					rule.prerequisites.emplace_back(prerequisite["id"].get_integer(), prerequisite["state"].get_integer());
				// These quizzes complete through dialogue. A quest with external
				// objectives remains open-book until objective tracking exists.
				for (auto requirement : finish)
					if (requirement.name() != "npc") rule.completion_ready = false;
				result.push_back(std::move(rule));
			}
			return result;
		}();
		return select_quest_marker(npc_id, job, rules, [&](int16_t id)
		{
			return completed.count(id) ? 2 : started.count(id) ? 1 : 0;
		}, level);
	}

	bool QuestLog::is_started(int16_t qid)
	{
		return started.count(qid) > 0;
	}

	int16_t QuestLog::get_last_started()
	{
		auto qend = started.end();
		qend--;

		return qend->first;
	}
}
