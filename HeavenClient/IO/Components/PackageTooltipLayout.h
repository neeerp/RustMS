#pragma once

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// Where v83's Cash Shop package tooltip (FUN_008efcb7) puts things: the
	// package's name, its description, a 53 px row per content, and a footer
	// with the contents' total price.
	namespace package_tooltip
	{
		constexpr int16_t MIN_WIDTH = 260;
		// A content's name is this much narrower than the box.
		constexpr int16_t NAME_MARGIN = 100;
		constexpr int16_t ROW_HEIGHT = 53;
		// The description's top, and its lines' pitch.
		constexpr int16_t DESC_Y = 35;
		constexpr int16_t LINE_HEIGHT = 16;
		constexpr int16_t ICON_X = 13;
		constexpr int16_t NAME_X = 55;

		// The box's width, from the widths of its contents' names.
		inline int16_t width(const std::vector<int16_t>& name_widths)
		{
			int16_t width = MIN_WIDTH;

			for (int16_t name_width : name_widths)
				width = std::max<int16_t>(width, name_width + NAME_MARGIN);

			return width;
		}

		// The description takes a line even when there is none.
		inline int16_t desc_height(int16_t lines)
		{
			return LINE_HEIGHT * std::max<int16_t>(1, lines);
		}

		// The top of row i, which is also where the footer goes for i = rows.
		inline int16_t row_y(size_t i, int16_t desc_height)
		{
			return static_cast<int16_t>(ROW_HEIGHT * i + desc_height + 30);
		}

		// mount_height is the height of the first mount content's
		// description, which v83 measures and adds without drawing.
		inline int16_t height(size_t rows, int16_t desc_height, int16_t mount_height)
		{
			return static_cast<int16_t>(ROW_HEIGHT * rows + 65 + desc_height + mount_height);
		}

		// A permanent content's name and price sit 9 px lower, where a timed
		// one has its period.
		inline int16_t name_y(int16_t row_y, int32_t period)
		{
			return row_y + (period != 0 ? 9 : 18);
		}

		inline int16_t price_y(int16_t row_y, int32_t period)
		{
			return name_y(row_y, period) + 16;
		}

		inline int16_t period_y(int16_t row_y)
		{
			return row_y + 40;
		}

		inline std::string price(int16_t count, int32_t nx)
		{
			std::string text = std::to_string(nx) + " NX";

			return count >= 2 ? std::to_string(count) + " " + text : text;
		}

		// The discount badge's percentage (FUN_008ef68f): all of it when
		// nothing is left to pay.
		inline int32_t percent(int32_t original, int32_t discounted)
		{
			if (discounted == 0)
				return 100;

			return (original - discounted) * 100 / original;
		}

		// The badge's digits: hundreds and tens only when there are any, the
		// ones unless the percentage is negative.
		inline std::vector<int32_t> digits(int32_t percent)
		{
			std::vector<int32_t> digits;

			if (percent / 100 > 0)
				digits.push_back(percent / 100);

			if (percent / 10 > 0)
				digits.push_back(percent % 100 / 10);

			if (percent >= 0)
				digits.push_back(percent % 10);

			return digits;
		}
	}
}
