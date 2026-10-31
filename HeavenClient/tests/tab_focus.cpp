#include "../IO/Components/TabFocus.h"
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
		using ms::tab_focus::next;

		auto all = [](size_t) { return true; };
		auto not_two = [](size_t index) { return index != 2; };
		auto only_one = [](size_t index) { return index == 1; };
		auto none = [](size_t) { return false; };

		require(next(0, 4, false, all) == 1u, "Tab moves to the next control");
		require(next(3, 4, false, all) == 0u, "Tab wraps from the last to the first");
		require(next(1, 4, true, all) == 0u, "Shift+Tab moves to the control before");
		require(next(0, 4, true, all) == 3u, "Shift+Tab wraps from the first to the last");

		require(next(1, 4, false, not_two) == 3u, "Tab passes a control that takes no focus");
		require(next(3, 4, true, not_two) == 1u, "so does Shift+Tab");

		require(!next(1, 4, false, only_one), "nothing else takes focus, so it stays put");
		require(!next(0, 4, true, none), "nothing takes focus at all");

		require(next(std::nullopt, 4, false, all) == 0u, "from nowhere, Tab starts at the first");
		require(next(std::nullopt, 4, true, all) == 3u, "and Shift+Tab at the last");
		require(next(std::nullopt, 4, false, not_two) == 0u, "from nowhere the first may take it");
		require(!next(std::nullopt, 4, false, none), "or none of them");

		require(!next(0, 1, false, all), "a lone control keeps the keys");
		require(next(std::nullopt, 1, false, all) == 0u, "but takes them from nowhere");
		require(!next(std::nullopt, 0, false, all), "and there is nothing to take them with no controls");

		std::cout << "Tab focus passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
