#include "../Gameplay/MapleMap/SwimArea.h"

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
		using namespace ms::swim_area;

		// Nautilus Harbor's (120000000) water, and the Generator Room's (120000301).
		std::vector<Rect> harbor = { { -606, 5318, 207, 302 } };
		std::vector<Rect> two = { { -606, 5318, 207, 302 }, { -400, 401, -45, 375 } };

		require(contains(harbor, 0, 250), "in the water");
		require(contains(harbor, -606, 207) && contains(harbor, 5318, 302), "on its edges");
		require(!contains(harbor, 0, 206) && !contains(harbor, 0, 303), "above and below it");
		require(!contains(harbor, -607, 250) && !contains(harbor, 5319, 250), "beside it");
		require(contains(two, 0, -45) && contains(two, 5000, 300), "in either of two");
		require(!contains({}, 0, 0), "a map with none");

		require(swimming(true, {}, 0, 0), "a swim map is water everywhere");
		require(swimming(false, harbor, 0, 250) && !swimming(false, harbor, 0, 100), "else only its areas");

		std::cout << "swim area tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
