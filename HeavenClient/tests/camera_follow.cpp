#include "../Gameplay/Camera.h"
#include <cmath>
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

		Camera camera;
		camera.set_view({-5000, 5000}, {-5000, 5000});
		camera.set_position({0, 0});
		require(camera.position() == Point<int16_t>(400, 350), "the view centres 50 px above the player's feet");

		camera.update({28, -28});
		require(camera.position() == Point<int16_t>(400, 350), "the view stays still within 28 px");

		// 1/400 + 1000 ms / 100000 = 1/80: one second closes 400 px to 80.
		for (int tick = 0; tick < 125; tick++)
			camera.update({400, 0});

		require(std::abs(camera.realposition(1.0f).x() - (400 - 320)) < 0.5, "a second closes a 400 px gap to 80 px");

		for (int tick = 0; tick < 125 * 60; tick++)
			camera.update({400, 0});

		double gap = camera.realposition(1.0f).x();
		require(gap > 27.9 && gap <= 28.0, "the view stops 28 px short of the player");

		// The Resting Spot, Pig Park's view range is narrower than the view.
		camera.set_view({-1199, -421}, {-805, 590});
		camera.set_position({-1100, 200});
		int16_t centred = camera.position().x();
		camera.update({-500, 200});
		require(camera.position().x() == centred && centred == 400 + 810, "a map narrower than the view is centred");

		// A shake moves the centre, inside the map's range.
		camera.set_view({-5000, 5000}, {-5000, 5000});
		camera.set_position({0, 0});
		camera.shake({10, -20});
		camera.update({0, 0});
		require(camera.position() == Point<int16_t>(390, 370), "a shake moves the view's centre");
		camera.shake({});
		camera.update({0, 0});
		require(camera.position() == Point<int16_t>(400, 350), "the next offset replaces the last");
		camera.set_view({-1199, -421}, {-805, 590});
		camera.set_position({-1100, 200});
		camera.shake({30, 30});
		camera.update({-1100, 200});
		require(camera.position().x() == centred, "the map's range keeps a shake in");

		std::cout << "Camera follow passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
