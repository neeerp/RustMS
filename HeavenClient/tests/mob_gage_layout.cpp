#include "../IO/UITypes/MobGageLayout.h"
#include <iostream>
#include <limits>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::mob_gage;

		// Without a minimap the bar takes the screen; half the HP fills half of it.
		Layout half = layout(0, 33000000, 66000000);
		require(half.width == 758 && half.fill == 379, "half of 758 columns");
		require(half.empty_from == 379 && half.empty == 379, "the rest is empty");
		require(!half.empty_left && half.right_x == 797 && !half.full_right, "the empty colour caps the right");

		// A 150 px minimap shortens it, and the fill truncates.
		Layout beside = layout(150, 1, 3);
		require(beside.width == 608 && beside.fill == 202, "a third of 608 truncates to 202");
		require(beside.right_x == 647, "the right cap ends the trough");

		Layout full = layout(0, 20000, 20000);
		require(full.fill == 758 && full.empty == 0 && full.full_right, "a full bar caps in its own colour");

		// Nearly dead rounds down to nothing, and the empty colour takes the left cap too.
		Layout last = layout(0, 1, 20000);
		require(last.fill == 0 && last.empty_left && last.empty == 758 && !last.full_right, "an empty bar");

		// Healed past its max, the fill runs past the trough and the right cap follows it.
		Layout over = layout(0, 30000, 20000);
		require(over.fill == 1137 && over.empty == 0 && over.empty_from == 1137, "the fill overruns");
		require(over.right_x == 1176 && !over.full_right, "an overrun caps in the empty colour");

		// v83 gets INT_MIN for a fill an int can't hold, so it shows empty.
		require(fill(5, 0, 758) == std::numeric_limits<int32_t>::min(), "no max HP");
		require(fill(2000000000, 1, 758) == std::numeric_limits<int32_t>::min(), "too large");
		require(layout(0, 5, 0).empty_left && layout(0, 5, 0).empty == 758, "an unfilled bar");

		// The icon's hit box is inclusive and a little larger than the icon.
		require(over_icon(150, 25, 25, 153, 5) && over_icon(150, 25, 25, 180, 30), "the corners");
		require(!over_icon(150, 25, 25, 152, 10) && !over_icon(150, 25, 25, 181, 10), "left and right of it");
		require(!over_icon(150, 25, 25, 160, 4) && !over_icon(150, 25, 25, 160, 31), "above and below it");
		require(over_icon(0, 0, 0, 4, 5) && !over_icon(0, 0, 0, 4, 6), "no icon leaves a sliver");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
