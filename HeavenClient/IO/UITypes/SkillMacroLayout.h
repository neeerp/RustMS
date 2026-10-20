#pragma once

#include <cstddef>
#include <cstdint>

// Where the cursor is in the skill macro window (v83 FUN_008a97c4), measured
// from the window's top left. The window shows ROWS macros from `scroll`.
namespace ms::skill_macro_layout
{
	constexpr size_t ROWS = 3;
	constexpr int16_t ROW_PITCH = 45, COLUMN_PITCH = 37, ICON_SIZE = 32;
	// The first row's first skill, and its macro's icon.
	constexpr int16_t SKILL_X = 20, SKILL_Y = 56, MACRO_ICON_X = 146;
	// The column of a macro's icon.
	constexpr int16_t ICON_COLUMN = 3;

	struct Hit
	{
		bool hit;
		// The macro: scroll + the row under the cursor, or 0 when no row is,
		// which v83 still selects on a click.
		size_t row;
		// A skill slot 0 to 2, ICON_COLUMN, or -1.
		int16_t column;
	};

	// Both ends of each band are in it. The icon column counts only when
	// `icon_column`; a skill is dropped only in a slot.
	inline Hit hit_test(int16_t x, int16_t y, size_t scroll, bool icon_column)
	{
		Hit hit{ false, 0, -1 };

		size_t k = 0;

		while (k < ROWS && !(y >= SKILL_Y + ROW_PITCH * static_cast<int16_t>(k)
				&& y <= SKILL_Y + ROW_PITCH * static_cast<int16_t>(k) + ICON_SIZE))
			k++;

		if (k == ROWS)
			return hit;

		hit.row = scroll + k;

		for (int16_t j = 0; j < 3; j++)
		{
			int16_t left = SKILL_X + COLUMN_PITCH * j;

			if (x >= left && x <= left + ICON_SIZE)
			{
				hit.hit = true;
				hit.column = j;

				return hit;
			}
		}

		if (icon_column && x >= MACRO_ICON_X && x <= MACRO_ICON_X + ICON_SIZE)
		{
			hit.hit = true;
			hit.column = ICON_COLUMN;
		}

		return hit;
	}
}
