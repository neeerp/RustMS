#include "../Gameplay/Combat/DamageLayout.h"
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
		using damage_layout::Digit;
		using damage_layout::Placed;

		// "123" in red: NoRed1/1, then NoRed0/2 and NoRed0/3.
		std::vector<Digit> digits = { { 25, 37, 12 }, { 29, 33, 14 }, { 28, 32, 14 } };
		std::vector<Placed> placed;

		int16_t width = damage_layout::place(digits, { 0, 0, 1 }, false, placed);
		require(width == 66, "the canvas ends at the last digit's right");
		require(placed[0].left == 0 && placed[0].top == 16, "the first digit's bottom is 4 px above the canvas's");
		require(placed[1].left == 18 && placed[1].top == 23, "each digit overlaps 3/5 of the last one's right part");
		require(placed[2].left == 38 && placed[2].top == 20, "a drop lowers an even digit by 1 px");

		width = damage_layout::place(digits, { 1, 1, 0 }, true, placed);
		require(width == 96 && placed[0].left == 30, "a critical number starts right of its banner");
		require(placed[0].top == 15 && placed[1].top == 22, "drops apply to odd digits too");

		require(damage_layout::shows_number(5221004), "a hit shows its number");
		require(!damage_layout::shows_number(5221009) && !damage_layout::shows_number(1221011),
				"Hypnotize's and Heaven's Hammer's don't");

		require(damage_layout::left(66) == -33 && damage_layout::left(67) == -33, "the canvas is centred, rounding left");

		require(damage_layout::opacity(0) == 1.0f && damage_layout::opacity(400) == 1.0f, "opaque for 400 ms");
		require(damage_layout::opacity(700) == 0.5f && damage_layout::opacity(1000) == 0.0f, "then fades over 600 ms");
		require(damage_layout::rise(500) == 15.0f && damage_layout::rise(2000) == 30.0f, "rises 30 px over a second");

		require(damage_layout::mob_offset(0, false) == 15, "the first line is 15 px above the head");
		require(damage_layout::mob_offset(2, false) == 75, "lines are 30 px apart");
		require(damage_layout::mob_offset(2, true) == 45, "Demolition's are 15 px apart");

		std::cout << "Damage layout passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
