#pragma once

#include <algorithm>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// What a map tooltip lists and how big its frame is, worked out from
	// scratch for each map so showing one again never changes it.
	namespace map_tooltip
	{
		constexpr size_t MAX_LIFE = 10;
		constexpr int16_t MIN_WIDTH = 166;
		constexpr int16_t BOTTOM_PADDING = 8;
		// The description sits this much inside the frame.
		constexpr int16_t DESC_MARGIN = 17;
		constexpr int16_t DESC_GAP = 4;
		constexpr int16_t LINE_GAP = 2;

		struct Life
		{
			std::vector<std::string> mobs, npcs;
		};

		struct Size
		{
			int16_t width, height;
		};

		// The mobs and NPCs of a map's life ("m" or "n", then the label), up
		// to MAX_LIFE of each.
		template<class Entries>
		Life pick_life(const Entries& entries)
		{
			Life life;

			for (const auto& entry : entries)
			{
				const std::pair<std::string, std::string>& kind_label = entry.second;
				auto& list = kind_label.first == "m" ? life.mobs : life.npcs;

				if ((kind_label.first == "m" || kind_label.first == "n") && list.size() < MAX_LIFE)
					list.push_back(kind_label.second);
			}

			return life;
		}

		// The width the description wraps at.
		inline int16_t wrap_width(int16_t name_width)
		{
			return std::max(MIN_WIDTH, name_width);
		}

		// The frame's size; desc_height is 0 without a description.
		inline Size size(int16_t name_width, int16_t name_height, int16_t desc_height, const Life& life,
						 int16_t line_height)
		{
			Size size = { wrap_width(name_width), name_height };

			if (desc_height > 0)
			{
				size.width += DESC_MARGIN;
				size.height += desc_height + DESC_GAP;
			}

			size_t lines = life.mobs.size() + life.npcs.size();
			size.height += static_cast<int16_t>(lines * (line_height + LINE_GAP));

			if (desc_height == 0 && lines > 0)
				size.height += BOTTOM_PADDING;

			return size;
		}
	}
}
