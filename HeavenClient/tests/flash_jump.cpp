#include "../Gameplay/Combat/FlashJump.h"
#include <cstring>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::flash_jump;

	try
	{
		require(jumps(4111006) && jumps(14101004), "both Flash Jumps jump");
		require(!jumps(11101005) && !jumps(2101002), "Soul Rush and Teleport don't");

		Impulse first = impulse(1, false);
		require(first.vx == 350.0 && first.vy == -250.0, "level 1 pushes 350 forward and 250 up");
		require(impulse(3, false).vx == 350.0, "levels 1-3 push the same");
		Impulse last = impulse(20, true);
		require(last.vx == -550.0 && last.vy == -350.0, "level 20 pushes 550 back and 350 up when facing left");

		require(may_jump(false, false, false, false, -1.0), "rising through the air");
		require(may_jump(false, false, false, false, 0.0), "at the top of a jump");
		require(!may_jump(false, false, false, false, 1.0), "not once falling");
		require(may_jump(false, false, false, true, 1.0), "falling through water is fine");
		require(!may_jump(true, false, false, false, 0.0), "not from the ground");
		require(!may_jump(false, true, false, false, 0.0), "not from a rope");
		require(!may_jump(false, false, true, false, -1.0), "once until landing");

		require(push_level(4111006, 20) == 20 && push_level(SOUL_RUSH, 10) == 20, "Soul Rush pushes at twice its level");
		require(may_rush(true, false, false, 1500, 1500), "Soul Rush from the ground once ready");
		require(!may_rush(false, false, false, 1500, 0), "not from the air");
		require(!may_rush(true, true, false, 1500, 0), "not from a rope");
		require(!may_rush(true, false, false, 1499, 1500), "not before its delay is up");
		require(std::strcmp(effect(SOUL_RUSH), "SoulRush") == 0, "Soul Rush's effect");
		require(std::strcmp(effect(4111006), "Flying") == 0, "a Hermit's effect");
		require(std::strcmp(effect(14101004), "Flying1") == 0, "a Night Walker's effect");
		require(std::strcmp(remote_effect(411), "Flying") == 0, "another Hermit's");
		require(std::strcmp(remote_effect(1411), "Flying1") == 0, "another Night Walker's");
		require(std::strcmp(remote_effect(1111), "SoulRush") == 0, "a Dawn Warrior's Soul Rush");
		require(remote_effect(400) == nullptr && remote_effect(420) == nullptr, "nobody else's");

		std::cout << "Flash Jump passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
