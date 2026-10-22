#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// Blessing of the Fairy: the account's highest other character lends
	// its levels, and the skill's tooltip names it (FUN_008f25d0, 0x8f29c8).
	namespace fairy_blessing
	{
		inline bool is_blessing(int32_t skill_id)
		{
			return skill_id == 12 || skill_id == 10000012 || skill_id == 20000012 || skill_id == 20010012;
		}

		// What the tooltip shows at a level past 0 instead of the
		// description: StringPool 0x1449, both %s the linked name.
		inline std::string description(const std::string &name)
		{
			return "Spirits who receive #c" + name +
				   "#'s energy will give your character a boost. 1 Skill Point will be granted each time #c" + name +
				   "#'s level increases by 10.";
		}
	}
}
