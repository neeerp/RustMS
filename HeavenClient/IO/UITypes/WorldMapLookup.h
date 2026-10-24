#pragma once

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <functional>
#include <string>
#include <vector>

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

		// The world map v83 opens to find an NPC (9ea1e1), given its maps
		// (Etc/NpcLocation.img), the world maps each one's MapLink leads to,
		// and whether a world map's MapList has a map. From WorldMap.img it
		// goes down into the first link whose world map has all of the hits
		// its level had, counted at the top, until a world map has no links
		// or none of them has all. Empty when no world map below the top
		// has any of the maps.
		inline std::string find_npc(const std::vector<int32_t> &maps,
			const std::function<std::vector<std::string>(const std::string&)> &links,
			const std::function<bool(const std::string&, int32_t)> &lists)
		{
			std::string current = "WorldMap";
			int32_t total = 0;

			for (int depth = 0; depth < 8; depth++)
			{
				std::vector<std::string> children = links(current);

				if (children.empty())
					break;

				std::vector<int32_t> hits(children.size());

				for (size_t i = 0; i < children.size(); i++)
					for (int32_t map : maps)
						if (lists(children[i], map))
						{
							hits[i]++;

							if (depth == 0)
								total++;
						}

				if (total == 0)
					return "";

				auto all = std::find(hits.begin(), hits.end(), total);

				if (all == hits.end())
					break;

				current = children[all - hits.begin()];
			}

			return current;
		}
	}
}
