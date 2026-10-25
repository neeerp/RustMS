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

		require(player_gauge(30000, 30000) == GAUGE_WIDTH && player_gauge(0, 500) == 0, "the player's gauge goes with HP");
		require(player_gauge(250, 500) == 152 && player_gauge(1, 500) == 0, "rounding down");
		require(player_gauge(10, 0) == 0, "and is empty without a maximum");
		require(PLAYER_GAUGE_RIGHT - player_gauge(250, 500) == 166, "losing HP from its left");
		require(monster_gauge(100) == GAUGE_WIDTH && monster_gauge(50) == 152 && monster_gauge(0) == 0,
				"the monster's goes with the boss's percent");
		require(monster_gauge(120) == GAUGE_WIDTH, "kept to 100");

		require((clock_digits(599) == std::array<int32_t, 4>{ 0, 9, 5, 9 }), "minutes and seconds, two digits each");
		require((clock_digits(0) == std::array<int32_t, 4>{ 0, 0, 0, 0 }), "00:00 at the end");
		require((clock_digits(6000 * 60 + 61) == std::array<int32_t, 4>{ 0, 1, 0, 1 }), "only the minutes' last two digits");

		Countdown clock;
		require(!clock.is_shown() && !clock.tick(5000), "no clock until one comes");
		clock.start(1000, 600);
		require(clock.is_shown() && !clock.has_digits(), "a clock shows its bar");
		require(!clock.tick(1000) && !clock.has_digits(), "and no digits while it still reads its seconds");
		require(!clock.tick(1001) && clock.has_digits() && clock.get_reading() == 599, "then whole seconds left, truncated");
		require(!clock.tick(600000) && clock.get_reading() == 1, "down to the last");
		require(clock.tick(600001) && clock.get_reading() == 0, "time is over once less than a second is left");
		require(!clock.tick(700000) && clock.get_reading() == 0 && clock.is_shown(), "and it stays at 0");
		clock.start(0, 0);
		require(clock.tick(0) && !clock.has_digits(), "a clock of 0 is over at once, showing only its bar");
		clock.start(0, 30);
		clock.start(0, 90);
		require(!clock.tick(1000) && clock.get_reading() == 89, "another clock takes over");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	std::cout << "dojo_energy tests passed" << std::endl;
	return 0;
}
