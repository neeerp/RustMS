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
#include "QuestCatalog.h"
#include <algorithm>

namespace ms
{
	void QuestLog::add_started(int16_t qid, const std::string &qdata)
	{
		bool fresh = started.count(qid) == 0;
		started[qid] = qdata;
		++marker_revision;

		if (fresh && std::find(dismissed.begin(), dismissed.end(), qid) == dismissed.end())
		{
			const QuestInfo* quest = QuestCatalog::get().find(qid);

			if (quest && (!quest->end_mobs.empty() || !quest->end_items.empty()))
				track(qid);
		}
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
		tracking.erase(std::remove(tracking.begin(), tracking.end(), qid), tracking.end());
		++marker_revision;
	}

	void QuestLog::remove(int16_t qid)
	{
		started.erase(qid);
		in_progress.erase(qid);
		completed.erase(qid);
		tracking.erase(std::remove(tracking.begin(), tracking.end(), qid), tracking.end());
		++marker_revision;
	}

	const std::vector<int16_t>& QuestLog::tracked() const { return tracking; }

	bool QuestLog::is_tracked(int16_t id) const
	{
		return std::find(tracking.begin(), tracking.end(), id) != tracking.end();
	}

	bool QuestLog::track(int16_t id)
	{
		if (is_tracked(id))
			return true;

		if (tracking.size() >= MAX_TRACKED || started.count(id) == 0)
			return false;

		tracking.push_back(id);
		dismissed.erase(std::remove(dismissed.begin(), dismissed.end(), id), dismissed.end());
		++marker_revision;
		return true;
	}

	void QuestLog::untrack(int16_t id)
	{
		if (!is_tracked(id))
			return;

		tracking.erase(std::remove(tracking.begin(), tracking.end(), id), tracking.end());
		dismissed.push_back(id);
		++marker_revision;
	}

	void QuestLog::clear()
	{
		started.clear(); in_progress.clear(); completed.clear();
		tracking.clear(); dismissed.clear();
		++marker_revision;
	}
	bool QuestLog::is_completed(int16_t id) const { return completed.count(id) != 0; }
	const std::map<int16_t, std::string>& QuestLog::started_quests() const { return started; }
	const std::map<int16_t, int64_t>& QuestLog::completed_quests() const { return completed; }
	uint64_t QuestLog::revision() const { return marker_revision; }

	int16_t QuestLog::state(int16_t id) const { return completed.count(id) ? 2 : started.count(id) ? 1 : 0; }

	std::vector<int16_t> QuestLog::available_quests(int16_t job, int16_t level, bool all_levels) const
	{
		std::vector<int16_t> result;
		for (const auto& quest : QuestCatalog::get().quests())
			if (quest_available(quest, job, level, [&](int16_t id) { return state(id); }, all_levels))
				result.push_back(quest.id);
		return result;
	}

	QuestMarker QuestLog::npc_marker(int32_t npc_id, int16_t job, int16_t level,
		const std::function<int32_t(int32_t)>& count) const
	{
		return select_quest_marker(npc_id, job, QuestCatalog::get().quests(), [&](int16_t id) { return state(id); }, level, count,
			[&](int16_t id)
			{
				auto iter = started.find(id);
				return iter == started.end() ? std::string() : iter->second;
			});
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
