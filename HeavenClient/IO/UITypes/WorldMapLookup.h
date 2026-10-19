#pragma once

#include <cstdio>
#include <functional>
#include <string>

namespace ms
{
	namespace world_map
	{
		inline std::string name(int number)
		{
			char buffer[16];
			std::snprintf(buffer, sizeof(buffer), "WorldMap%03d", number);

			return buffer;
		}

		// The world map v83 shows a field on (9e9ce4), given which world
		// maps list it in their MapList: the first of WorldMap000, 010 to
		// 100 that does, unless one of the ten after it (011 to 020 for
		// 010) or WorldMap140 to 142 does too. Empty when none does, and
		// the world map isn't available there. WorldMap.img itself isn't
		// looked at.
		inline std::string find(const std::function<bool(const std::string&)> &lists)
		{
			std::string found;

			for (int top = 0; top <= 100 && found.empty(); top += 10)
			{
				if (!lists(name(top)))
					continue;

				for (int sub = top + 1; sub <= top + 10; sub++)
					if (lists(name(sub)))
						return name(sub);

				found = name(top);
			}

			for (int extra = 140; extra <= 142; extra++)
				if (lists(name(extra)))
					return name(extra);

			return found;
		}
	}
}
