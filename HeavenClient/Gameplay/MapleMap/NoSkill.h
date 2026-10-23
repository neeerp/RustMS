#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

namespace ms
{
	// The skills a map's noSkill node takes away (CField+0x770): every skill of a job advancement in its class
	// list, and each skill in its skill list. Only a skill the player uses is refused, with no packet; the
	// regular attack goes on (FUN_00587f5f from DoActiveSkill FUN_00966f7a).
	namespace no_skill
	{
		// The job advancement of a job, 0 for none (FUN_004e8f66): 1 for a beginner line or a first job and
		// Evan's first, 2 to 4 for the second to fourth, and Evan's 5 to 10.
		inline int32_t advancement(int32_t job)
		{
			if (job % 100 == 0 || job == 2001)
				return 1;

			int32_t step = job % 10 + 2;

			if (step > 1 && (step < 5 || (step < 11 && job / 100 == 22)))
				return step;

			return 0;
		}

		struct Lists
		{
			std::vector<int32_t> classes;
			std::vector<int32_t> skills;
		};

		// Whether the map takes the skill away: its advancement is in the class list, or the skill is in the
		// skill list. A job with no advancement matches no class.
		inline bool forbids(const Lists &lists, int32_t skill)
		{
			int32_t step = advancement(skill / 10000);

			if (step != 0 && std::find(lists.classes.begin(), lists.classes.end(), step) != lists.classes.end())
				return true;

			return std::find(lists.skills.begin(), lists.skills.end(), skill) != lists.skills.end();
		}
	}
}
