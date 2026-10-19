#include "../Gameplay/MapleMap/ObstacleRules.h"
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
		using namespace ms::obstacle;

		// The clock directions in the data, taken as radians.
		require(push(0, false) == Vec{ 0, 0 } && push(0, true) == Vec{ 0, 0 }, "no direction");
		require(push(3, false) == Vec{ 400, 0 }, "3 pushes right");
		require(push(6, false) == Vec{ -179, 357 }, "6");
		require(push(9, false) == Vec{ -239, -320 }, "9");
		require(push(12, false) == Vec{ 393, -70 }, "12");
		require(push(3, true) == Vec{ -400, 0 } && push(9, true) == Vec{ 239, -320 }, "a flip mirrors the push");

		// The frame's box goes on the obj, mirrored by a flip.
		Box out{};
		require(box(100, 200, { -10, -40 }, { 30, 0 }, false, out) && out == Box{ 90, 160, 130, 200 }, "a box");
		require(box(100, 200, { -10, -40 }, { 30, 0 }, true, out) && out == Box{ 70, 160, 110, 200 }, "flipped");
		require(!box(0, 0, { 0, 0 }, { 0, 0 }, false, out), "a frame without lt and rb");
		require(!box(0, 0, { 0, 0 }, { 4, 4 }, false, out), "too small both ways");
		require(box(0, 0, { 0, 0 }, { 4, 5 }, false, out), "tall enough");

		// Boxes that only touch don't meet.
		Box body{ 0, 0, 10, 10 };
		require(body.meets(Box{ 9, 9, 20, 20 }) && !body.meets(Box{ 10, 0, 20, 10 }), "IntersectRect");

		// Within a tenth either way, from rand().
		require(roll(100, 0) == 90 && roll(100, 20) == 110 && roll(100, 21) == 90, "100 rolls 90 to 110");
		require(roll(10, 7) == 10 && roll(1, 7) == 1 && roll(0, 7) == 0, "10 or less stays");
		require(roll(11, 2) == 12 && roll(11, 3) == 10, "11 rolls 10 to 12");

		// Test of Agility's menhirs leave the player on 100 HP.
		require(forced(50000, 900, 100) == 800 && forced(50000, 50, 100) == 50 && forced(50000, 100, 100) == 100,
				"forceHP");
		require(forced(57, 900, -1) == 57, "no forceHP");

		Trap stun;
		stun.disease = 123;
		stun.level = 3;
		require(code(5, stun) == 0x7b03 && code(0, stun) == 0, "a disease with a hit");
		stun.level = 0;
		require(code(5, stun) == 0, "no level, no disease");

		// Back against the step, and up unless rising.
		require(fallback(10, 0, 12, 0) == Vec{ -270, -270 }, "walking right");
		require(fallback(10, 0, 8, 5) == Vec{ 270, -270 }, "falling left");
		require(fallback(10, 5, 10, 0) == Vec{ 0, 0 }, "rising");
		require(fallback(10, 0, 10, 0) == Vec{ 0, -270 }, "standing");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
