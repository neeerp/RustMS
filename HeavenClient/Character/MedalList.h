#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

// The rows of v83's title (medal) window (FUN_008427db, FUN_00841b32,
// report b147). Its tabs are Basic, every title obtained, then one per
// QuestInfo medalCategory: 0 Job, 1 General, 2 Challenge and 3 Event.
// A category's tab lists the titles being challenged and the obtainable
// ones, each under a header that can fold them away.
namespace ms::medal_list
{
	// The quest's state (FUN_00a2a0be).
	enum State : uint8_t
	{
		OBTAINABLE,
		CHALLENGING,
		OBTAINED
	};

	// The headers (FUN_0084526f).
	enum Group : uint8_t
	{
		CHALLENGED,
		AVAILABLE,
		NUM_GROUPS
	};

	constexpr uint8_t BASIC = 0;
	constexpr uint8_t NUM_TABS = 5;

	struct Medal
	{
		int16_t quest;
		// QuestInfo medalCategory, 0 when absent.
		int32_t category;
		// The start check's lvmin.
		int32_t min_level;
		State state;
		// When it was obtained, a FILETIME.
		int64_t obtained_at;
	};

	// A header (`quest` < 0, with how many titles it has, folded or not) or
	// a title.
	struct Row
	{
		int16_t quest;
		Group group;
		int16_t count;
	};

	inline Group group_of(State state)
	{
		return state == CHALLENGING ? CHALLENGED : AVAILABLE;
	}

	// Tab `tab`'s rows: in a category's, the titles in each group by their
	// level (FUN_008493a7; v83 leaves ties to its sort, here by quest), and
	// in Basic the latest obtained first (FUN_00849622). `folded` has bit g
	// set for a folded group g.
	inline std::vector<Row> rows(std::vector<Medal> medals, uint8_t tab, uint8_t folded)
	{
		std::sort(medals.begin(), medals.end(), [tab](const Medal& a, const Medal& b)
		{
			if (tab == BASIC && a.obtained_at != b.obtained_at)
				return a.obtained_at > b.obtained_at;

			return a.min_level != b.min_level ? a.min_level < b.min_level : a.quest < b.quest;
		});

		std::vector<Row> out;

		if (tab == BASIC)
		{
			for (const Medal& medal : medals)
				if (medal.state == OBTAINED)
					out.push_back({ medal.quest, AVAILABLE, 0 });

			return out;
		}

		for (uint8_t g = 0; g < NUM_GROUPS; g++)
		{
			Group group = static_cast<Group>(g);
			std::vector<Row> titles;

			for (const Medal& medal : medals)
				if (medal.category == tab - 1 && medal.state != OBTAINED && group_of(medal.state) == group)
					titles.push_back({ medal.quest, group, 0 });

			if (titles.empty())
				continue;

			out.push_back({ -1, group, static_cast<int16_t>(titles.size()) });

			if (!(folded & (1 << g)))
				out.insert(out.end(), titles.begin(), titles.end());
		}

		return out;
	}
}
