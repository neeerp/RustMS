#include "../IO/UITypes/SkillMacroLayout.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::skill_macro_layout;

		Hit first = hit_test(20, 56, 0, true);
		require(first.hit && first.row == 0 && first.column == 0, "the first slot's corner");

		Hit last = hit_test(52 + 74, 88 + 90, 2, false);
		require(last.hit && last.row == 4 && last.column == 2, "both ends of a band count, from the scroll");

		Hit icon = hit_test(146, 101, 1, true);
		require(icon.hit && icon.row == 2 && icon.column == ICON_COLUMN, "a macro's icon");

		Hit no_icon = hit_test(146, 101, 1, false);
		require(!no_icon.hit && no_icon.row == 2 && no_icon.column == -1, "a drop takes no icon, but keeps the row");

		Hit gap = hit_test(54, 60, 2, true);
		require(!gap.hit && gap.row == 2, "between slots is still the row");

		Hit between = hit_test(20, 95, 2, true);
		require(!between.hit && between.row == 0 && between.column == -1, "between rows is macro 0, whatever the scroll");

		Hit below = hit_test(20, 179, 0, true);
		require(!below.hit && below.row == 0, "nothing below the last row");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "skill macro layout tests passed\n";
	return 0;
}
