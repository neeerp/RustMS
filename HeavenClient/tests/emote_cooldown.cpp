#include "../Gameplay/EmoteCooldown.h"

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
		ms::EmoteCooldown cooldown;

		require(cooldown.try_start(0), "the first emote is never held back");
		require(!cooldown.try_start(1), "a press right after an emote does nothing");
		require(!cooldown.try_start(1999), "the wait lasts 2 seconds");
		require(cooldown.try_start(2000), "an emote may start once 2 seconds have passed");
		require(!cooldown.try_start(3999), "an ignored press doesn't extend the wait");
		require(cooldown.try_start(4500), "presses after the wait work again");

		std::cout << "Emote cooldown passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
