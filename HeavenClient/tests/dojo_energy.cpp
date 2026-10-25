#include "../Gameplay/Combat/DojoEnergy.h"

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
		using namespace ms::dojo_energy;

		require(cover(0) == 77 && cover(FULL) == 0, "the cover hides all the strip at none, nothing when full");
		require(cover(5000) == 39 && cover(100) == 77 && cover(130) == 76, "and shrinks a pixel per 1/77");
		require(cover(-5) == 77 && cover(20000) == 0, "out of range values are kept in");

		require(secret_skill(0, Arrow::UP, false) == 1009, "Up casts Bamboo Rain");
		require(secret_skill(0, Arrow::UP, true) == 0, "once a stage");
		require(secret_skill(1100, Arrow::LEFT, true) == 10001010, "Left casts the job's Invincibility");
		require(secret_skill(2200, Arrow::RIGHT, false) == 20011011, "Right casts Power Explosion, Evan's 2001's");
		require(secret_skill(0, Arrow::NONE, false) == 0, "no arrow, no skill");

		using Modifier = Chord::Modifier;
		using Result = Chord::Result;

		Chord chord;
		require(chord.press(Modifier::CTRL) == Result::HOLD, "the first of the two is held back");
		require(chord.press(Modifier::ALT) == Result::CHORD, "the other completes the chord");
		require(chord.press(Modifier::ALT) == Result::HOLD, "and then starts over");
		require(chord.press(Modifier::ALT) == Result::HOLD, "the same one again stays held");
		require(chord.press(Modifier::CTRL) == Result::CHORD, "either order");
		require(chord.press(Modifier::CTRL) == Result::HOLD && chord.press(Modifier::NONE) == Result::PASS,
				"any other key lets it through");
		require(chord.press(Modifier::ALT) == Result::HOLD, "and doesn't count toward a chord");
		chord.reset();
		require(chord.press(Modifier::CTRL) == Result::HOLD, "nor does a key let through on its release");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	std::cout << "dojo_energy tests passed" << std::endl;
	return 0;
}
