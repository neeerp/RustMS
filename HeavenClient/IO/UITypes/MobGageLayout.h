#pragma once

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <limits>

// The boss HP bar along the top of the screen (FIELD_EFFECT 5, v83 0x5336ca), from UIWindow.img/MobGage.
// Everything is placed from the bar's left, which is the minimap's right edge (or the screen's left).
namespace ms::mob_gage
{
	// The icon's frame, then the trough from TROUGH_X, whose columns start at START. The fill's top is FILL_Y,
	// with its left cap at CAP_X.
	constexpr int16_t ICON_X = 5, ICON_Y = 6, TROUGH_X = 35, START = 39, CAP_X = 38, FILL_Y = 4;
	// The tip goes this far right of the icon (0x529b63).
	constexpr int16_t TIP_GAP = 10;

	struct Layout
	{
		// The trough's columns; its right end goes at START + width.
		int32_t width;
		// How many columns hp fills. It can be past width, or negative.
		int32_t fill;
		// The empty part's columns, from START + empty_from.
		int32_t empty_from;
		int32_t empty;
		// v83 caps an empty bar's left in the empty colour.
		bool empty_left;
		int32_t right_x;
		// The right cap is the fill's colour only on a full bar.
		bool full_right;
	};

	// v83 fills the bar in doubles and truncates with _ftol (0xa62018), which gives INT_MIN for what an int
	// can't hold, such as a maxhp of 0.
	inline int32_t fill(int32_t hp, int32_t maxhp, int32_t width)
	{
		double columns = static_cast<double>(hp) / maxhp * width;

		if (!(columns > std::numeric_limits<int32_t>::min() - 1.0 && columns < std::numeric_limits<int32_t>::max() + 1.0))
			return std::numeric_limits<int32_t>::min();

		return static_cast<int32_t>(std::trunc(columns));
	}

	// The bar beside a minimap `left` wide (0x534365): it would end at 793 on v83's 800 px screen.
	inline Layout layout(int16_t left, int32_t hp, int32_t maxhp)
	{
		Layout result;
		result.width = 758 - left;
		result.fill = fill(hp, maxhp, result.width);
		result.empty_from = std::max(result.fill, 0);
		result.empty = std::max(result.width - result.empty_from, 0);
		result.empty_left = result.fill <= 0;
		result.right_x = START + std::max(result.fill, result.width);
		result.full_right = result.fill == result.width;

		return result;
	}

	// Whether the cursor, on the screen, is over the mob's icon (0x529ac5). The bounds are inclusive and run a
	// few pixels past the icon.
	inline bool over_icon(int16_t left, int16_t icon_width, int16_t icon_height, int16_t x, int16_t y)
	{
		return x >= left + 3 && x <= left + icon_width + 5 && y >= 5 && y <= icon_height + 5;
	}
}
