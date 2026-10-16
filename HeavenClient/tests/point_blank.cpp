#include "../Gameplay/Combat/PointBlank.h"
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
		using namespace ms;
		using point_blank::range;

		Rectangle<int16_t> bow = range({ -30, 0, -81, 5 });
		require(bow.left() == -30 && bow.right() == 0, "a bow's swing T1 keeps its 30 px reach");
		require(bow.top() == -81 && bow.bottom() == 5, "the height stays as it is");

		Rectangle<int16_t> claw = range({ -74, -27, -28, -18 });
		require(claw.left() == -65 && claw.right() == -27, "a claw's stab O1 reaches 65 px, not 74");

		Rectangle<int16_t> thin = range({ -90, -70, -10, 0 });
		require(thin.left() == -71 && thin.right() == -70, "a range starting past 65 px keeps 1 px");

		std::cout << "Point blank passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
