#include "../IO/DoublePress.h"
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
		using ms::DoublePress;

		{
			DoublePress clicks;
			require(!clicks.press(1000, 100, 100), "the first press is a plain one");
			require(clicks.press(1090, 100, 100), "the second at once is a double click's");
			require(!clicks.press(1180, 100, 100), "a third is plain again");
			require(clicks.press(1270, 100, 100), "and a fourth makes another double click");
		}
		{
			DoublePress clicks;
			clicks.press(1000, 100, 100);
			require(clicks.press(1500, 100, 100), "the double click time counts");
		}
		{
			DoublePress clicks;
			clicks.press(1000, 100, 100);
			require(!clicks.press(1501, 100, 100), "a press after it is plain");
			require(clicks.press(1601, 100, 100), "and starts the time again");
		}
		{
			DoublePress clicks;
			clicks.press(1000, 100, 100);
			require(clicks.press(1100, 96, 103), "a press inside the box counts");
		}
		{
			DoublePress clicks;
			clicks.press(1000, 100, 100);
			require(!clicks.press(1100, 104, 100), "the box's right edge is out");
			require(!clicks.press(1200, 104, 108), "and its lower edge");
			require(clicks.press(1300, 104, 108), "each press out of it is a first one");
		}
		{
			DoublePress clicks;
			require(!clicks.press(0, 0, 0), "the very first press at the origin is plain");
			clicks.press(100, 0, 0);
			require(clicks.at_first(3, -4) && !clicks.at_first(4, 0), "where a release still counts");
		}
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "double press tests passed\n";
	return 0;
}
