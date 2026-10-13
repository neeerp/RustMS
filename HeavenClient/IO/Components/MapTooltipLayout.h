#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// How big a map tooltip's box is, worked out from scratch for each map so
	// showing one again never changes it. Measured from period footage.
	namespace map_tooltip
	{
		// The box is this much wider and taller than the text's lines.
		constexpr int16_t PADDING_X = 10;
		constexpr int16_t PADDING_Y = 7;
		// Descriptions wrap at this width, or the name's if that is wider.
		constexpr int16_t DESC_WIDTH = 200;
		constexpr int16_t DESC_GAP = 2;

		struct Size
		{
			int16_t width, height;
		};

		// The width the description wraps at.
		inline int16_t wrap_width(int16_t name_width)
		{
			return std::max(DESC_WIDTH, name_width);
		}

		// The box's size; desc_height is 0 without a description.
		inline Size size(int16_t name_width, int16_t name_height, int16_t desc_width, int16_t desc_height)
		{
			Size size = { std::max(name_width, desc_width), name_height };

			if (desc_height > 0)
				size.height += DESC_GAP + desc_height;

			size.width += 2 * PADDING_X;
			size.height += 2 * PADDING_Y;

			return size;
		}
	}
}
