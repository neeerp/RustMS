#include "../Gameplay/FameCooldown.h"

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
		ms::FameCooldown cooldown;

		require(cooldown.try_give(1000), "the first fame is never held back");
		require(!cooldown.try_give(1001), "a press right after giving fame sends nothing");
		require(!cooldown.try_give(300999), "the wait lasts 5 minutes");
		require(cooldown.try_give(301000), "fame may be given once 5 minutes have passed");
		require(!cooldown.try_give(600999), "a refused press doesn't extend the wait");
		require(cooldown.try_give(601000), "presses after the wait work again");

		std::cout << "Fame cooldown passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
