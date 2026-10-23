#pragma once

#include <cstdint>
#include <map>
#include <unordered_map>
#include <vector>

// The rules behind v83's SP Reset (5050001-5050004): which skill a level may leave and which it may go to
// (report 195 section 4). Each item resets into one advancement's skills; a level may come from that
// advancement or, while there is SP to spare, an earlier one.
namespace ms::sp_reset
{
	constexpr int32_t FIRST_ITEM = 5050001;
	constexpr int32_t LAST_ITEM = 5050004;

	inline bool is_item(int32_t item_id)
	{
		return item_id >= FIRST_ITEM && item_id <= LAST_ITEM;
	}

	// The advancement whose skills the item resets into, as an index into the job path: 0 for 5050001.
	inline int32_t item_tab(int32_t item_id)
	{
		return item_id % 10 - 1;
	}

	// A job's advancement (FUN_004e8f66): 1 for a first job (and a beginner), 2 to 4 after, 0 for the rest.
	inline int32_t advancement(int32_t job)
	{
		if (job % 100 == 0)
			return 1;

		int32_t advancement = job % 10 + 2;

		return advancement >= 2 && advancement <= 4 ? advancement : 0;
	}

	// The jobs a character advanced through, first job first (FUN_004a8c4f): a Hero's are 100, 110, 111, 112.
	inline std::vector<int32_t> job_path(int32_t job)
	{
		std::vector<int32_t> path;
		int32_t branch = job % 1000 / 100;

		if (branch == 0)
			return path;

		int32_t first = (branch + job / 1000 * 10) * 100;
		path.push_back(first);

		if (int32_t second = job % 100 / 10)
			for (int32_t n = 0; n <= job % 10; n++)
				path.push_back(first + second * 10 + n);

		return path;
	}

	// Not a beginner or GM, and advanced as far as the item's skills.
	inline bool may_use(int32_t job, int32_t tab)
	{
		int32_t branch = job % 1000 / 100;

		return branch >= 1 && branch <= 8 && advancement(job) > tab;
	}

	// The tab a skill is listed in: its job's place in the path, or -1.
	inline int32_t skill_tab(const std::vector<int32_t> &path, int32_t skill_id)
	{
		for (size_t i = 0; i < path.size(); i++)
			if (path[i] == skill_id / 10000)
				return static_cast<int32_t>(i);

		return -1;
	}

	// Skills with a master level only go up to it (FUN_004e8f04): 4th job,
	// and Evan's 9th and 10th stages (2217, 2218: FUN_004e8f66).
	inline bool has_master_level(int32_t skill_id)
	{
		int32_t job = skill_id / 10000;

		if (job / 100 == 22 || job == 2001)
			return job == 2217 || job == 2218;

		return job % 100 != 0 && job % 10 == 2;
	}

	struct Character
	{
		int32_t job;
		int32_t level;
		int32_t sp;
		// Every skill the character has, by id.
		std::map<int32_t, int32_t> levels;
	};

	inline int32_t level_of(const Character &c, int32_t skill_id)
	{
		auto iter = c.levels.find(skill_id);

		return iter == c.levels.end() ? 0 : iter->second;
	}

	// The SP put into earlier advancements' skills beyond what they could earn, which may move into `tab`'s
	// (FUN_008b2ab7, summed as FUN_008b2918 does).
	inline int32_t spare(const Character &c, int32_t tab)
	{
		int32_t sums[5] = {};

		for (auto [skill_id, level] : c.levels)
			if (int32_t of = advancement(skill_id / 10000); of >= 1 && of <= 4)
				sums[of] += level;

		int32_t adv = advancement(c.job);
		bool mage = c.job % 1000 / 100 == 2;
		int32_t first = (c.level - (mage ? 8 : 10)) * 3 - (mage ? 6 : 0) - c.sp - sums[2] - sums[3] - sums[4] + adv
						+ (adv == 4 ? 2 : 0) - 61;
		int32_t second = sums[2] - 121;
		int32_t third = sums[3] - 151;

		switch (tab)
		{
			case 1: return first;
			case 2: return first + second;
			case 3: return first + second + third;
			default: return 0;
		}
	}

	// Whether a skill the character has (`reqs_of` gives its `req`) needs `skill` at the level it is now
	// (FUN_008b29d4).
	template <typename ReqsOf>
	bool relied_on(const Character &c, int32_t skill_id, ReqsOf reqs_of)
	{
		int32_t level = level_of(c, skill_id);

		for (auto [other, other_level] : c.levels)
		{
			if (other_level < 1)
				continue;

			const std::unordered_map<int32_t, int32_t> &reqs = reqs_of(other);
			auto iter = reqs.find(skill_id);

			if (iter != reqs.end() && level <= iter->second)
				return true;
		}

		return false;
	}

	// Whether the first dialog lets a level leave `skill_id`, listed in `listed_tab`, for the item's `tab`
	// (FUN_008b2918).
	template <typename ReqsOf>
	bool may_take(const Character &c, int32_t skill_id, int32_t listed_tab, int32_t tab, ReqsOf reqs_of)
	{
		if (relied_on(c, skill_id, reqs_of) || level_of(c, skill_id) < 1)
			return false;

		return listed_tab == tab || spare(c, tab) > 0;
	}

	// Whether the second lets it go to `skill_id`, having left `from` (FUN_00a08e05 and FUN_008b5771). Its
	// `req` is met and it isn't at its max level, or the master level it has one; no SP is needed.
	inline bool may_give(const Character &c, int32_t skill_id, int32_t from, int32_t master_level,
						 int32_t max_level, const std::unordered_map<int32_t, int32_t> &reqs)
	{
		if (skill_id == from)
			return false;

		for (auto [req, req_level] : reqs)
			if (level_of(c, req) < req_level)
				return false;

		int32_t level = level_of(c, skill_id);

		if (has_master_level(skill_id) && master_level <= level)
			return false;

		return level < max_level;
	}
}
