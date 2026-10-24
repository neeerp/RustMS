#include "../IO/UITypes/GaugeSlide.h"
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
		using namespace ms::gauge_slide;

		require(gray(50, 50, 105) == 0 && gray(0, 50, 105) == 105, "a full gauge has no gray, an empty one all");
		require(gray(25, 50, 105) == 53 && gray(1, 3, 105) == 70, "the filled part truncates");
		require(gray(10, 0, 105) == 0, "a zero max shows the gauge full");

		Slide slide;
		slide.aim(40.0, 1000);
		require(slide.at(1000) == 40.0, "the first aim shows the gauge at once");

		slide.aim(100.0, 2000);
		require(slide.at(2000) == 40.0, "a change starts from what is shown");
		require(slide.at(2350) == 70.0, "it moves linearly");
		require(slide.at(2700) == 100.0 && slide.at(9000) == 100.0, "and arrives after 700 ms");

		slide.aim(0.0, 3000);
		require(slide.at(3350) == 50.0, "a gain slides the same way");

		slide.aim(50.0, 3350);
		require(slide.at(3350) == 50.0 && slide.at(4050) == 50.0, "a retarget starts from the shown width");

		slide.aim(20.0, 5000);
		slide.aim(20.0, 5350);
		require(slide.at(5350) == 35.0, "the same target again doesn't restart the slide");

		std::cout << "Gauge slide passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
