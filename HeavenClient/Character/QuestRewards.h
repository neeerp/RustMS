#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// Act.img/<id>/1: what completing a quest gives, as far as the server
	// supports it (no skills, closeness or items to choose from).
	struct QuestRewards
	{
		struct Item
		{
			int32_t id;
			int32_t count;
			// A bit per job branch (1 << job / 100); -1 is every job.
			int32_t jobs = -1;
			// 0 male, 1 female, 2 either.
			int32_t gender = 2;
			// 0 always given; above 0, one of these is given at random.
			int32_t prop = 0;

			bool applies_to(int16_t job, bool female) const
			{
				int32_t branch = job / 100;

				return ((jobs & (1 << branch)) != 0 || branch == 9) && (gender == 2 || gender == (female ? 1 : 0));
			}
		};

		int32_t meso = 0;
		int32_t exp = 0;
		// Fame (pop); a loss isn't listed.
		int32_t fame = 0;
		std::vector<Item> items;

		// The list v83 puts under the last completion page (FUN_0071808a),
		// or nothing when there is nothing to list or another quest follows.
		// "#w" shows it whole while the page above types out.
		std::string summary(int16_t job, bool female, bool next_quest) const
		{
			std::string given;
			bool random = false;

			for (const Item& item : items)
			{
				if (item.count <= 0 || !item.applies_to(job, female))
					continue;

				if (item.prop > 0)
				{
					random = true;
				}
				else
				{
					std::string id = std::to_string(item.id);
					given += "#v" + id + ":# #t" + id + ":# " + std::to_string(item.count) + "\r\n";
				}
			}

			if (next_quest || (given.empty() && !random && meso <= 0 && exp <= 0 && fame <= 0))
				return {};

			std::string text = "#w\r\n\r\n#fUI/UIWindow.img/QuestIcon/4/0#\r\n" + given + "\r\n";

			if (meso > 0)
				text += "#fUI/UIWindow.img/QuestIcon/7/0# " + std::to_string(meso) + " Meso\r\n";

			if (exp > 0)
				text += "#fUI/UIWindow.img/QuestIcon/8/0# " + std::to_string(exp) + " exp\r\n";

			if (fame > 0)
				text += "#fUI/UIWindow.img/QuestIcon/6/0# " + std::to_string(fame) + "\r\n";

			if (meso > 0 || exp > 0 || fame > 0)
				text += "\r\n";

			// Which random item is given is up to the server; v83 names none.
			if (random)
				text += "#FUI/UIWindow.img/QuestIcon/5/0#\r\n";

			return text + "#w";
		}
	};
}
