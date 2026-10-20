#include "../Gameplay/Combat/RecoilShot.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::recoil_shot;

	try
	{
		ms::flash_jump::Impulse first = backstep(1, false);
		require(first.vx == -250.0 && first.vy == -250.0, "level 1 pushes 250 back and 250 up");
		require(backstep(4, false).vx == -290.0 && backstep(4, false).vy == -270.0, "level 4 pushes 290 back and 270 up");
		ms::flash_jump::Impulse last = backstep(20, true);
		require(last.vx == 450.0 && last.vy == -350.0, "level 20 pushes 450 right and 350 up when facing left");

		std::cout << "Recoil Shot passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
