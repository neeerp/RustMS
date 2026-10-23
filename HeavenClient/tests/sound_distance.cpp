#include "../Audio/SoundDistance.h"
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
		using ms::sound_distance::volume;

		require(volume(0, 0) == 100, "a sound at the player is full");
		require(volume(249, 0) == 100 && volume(0, -249) == 100, "full within 250 px");
		require(volume(250, 0) == 99, "at 250 px the +0.001 under the root truncates 120 - 20 to 99");
		require(volume(300, 400) == 79, "500 px away is just under 80, so 79");
		require(volume(-626, 0) == 69, "the fall is truncated (69.92)");
		require(volume(1000, 0) == 40 && volume(0, 5000) == 40, "40 at and beyond 1000 px");

		std::cout << "sound_distance: ok\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << "sound_distance: " << e.what() << "\n";
		return 1;
	}
}
