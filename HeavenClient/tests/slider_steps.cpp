#include "../IO/Components/SliderSteps.h"
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
		using namespace ms::slider_steps;

		require(thumb_x(0, 10, 100) == 0, "the first step is at the left end");
		require(thumb_x(10, 10, 100) == 100, "the last step is at the right end");
		require(thumb_x(5, 10, 100) == 50, "steps are spread evenly");
		require(thumb_x(1, 3, 100) == 33 && thumb_x(2, 3, 100) == 67, "uneven steps round to the nearest pixel");

		require(nearest(-20, 10, 100) == 0, "left of the track is the first step");
		require(nearest(140, 10, 100) == 10, "right of the track is the last step");
		require(nearest(44, 10, 100) == 4 && nearest(45, 10, 100) == 5, "a thumb between steps snaps to the closer one");

		for (int16_t step = 0; step <= 7; step++)
			require(nearest(thumb_x(step, 7, 131), 7, 131) == step, "every step's place maps back to that step");

		require(thumb_x(3, 0, 100) == 0 && nearest(50, 0, 100) == 0, "a slider with no steps stays at the left");

		std::cout << "Slider steps passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
