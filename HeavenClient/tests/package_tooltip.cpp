#include "../IO/Components/PackageTooltipLayout.h"
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
		using namespace ms::package_tooltip;

		require(width({ 100, 150 }) == 260, "the box is at least 260 wide");
		require(width({ 100, 175 }) == 275, "a long name widens it by its width and 100");
		require(desc_height(0) == 16 && desc_height(2) == 32, "the description takes a line even when empty");
		require(row_y(0, 16) == 46 && row_y(2, 16) == 152, "rows are 53 apart under the description");
		require(height(3, 16, 0) == 240, "three rows with no description");
		require(height(1, 16, 32) == 166, "a mount's description adds height");
		require(name_y(46, 90) == 55 && price_y(46, 90) == 71, "a timed content's name and price");
		require(name_y(46, 0) == 64 && price_y(46, 0) == 80, "a permanent one's sit 9 lower");
		require(period_y(46) == 86, "the period line");

		require(price(1, 2100) == "2100 NX", "one of a content");
		require(price(11, 1500) == "11 1500 NX", "a count goes first");

		require(percent(1000, 1000) == 0, "no discount");
		require(percent(1000, 0) == 100, "all bonus");
		require(percent(3000, 2400) == 20, "a fifth off");
		require(percent(800, 3000) < 0, "dearer than the regular price");
		require(digits(0) == std::vector<int32_t>({ 0 }), "0 has a digit");
		require(digits(7) == std::vector<int32_t>({ 7 }), "one digit");
		require(digits(20) == std::vector<int32_t>({ 2, 0 }), "two digits");
		require(digits(100) == std::vector<int32_t>({ 1, 0, 0 }), "three digits");
		require(digits(-275).empty(), "a negative percentage has none");

		std::cout << "Package tooltip passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
