#include "../Gameplay/Combat/RemoteSpell.h"

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
		using namespace ms::remote_spell;

		require(start({ 100, 200 }, true) == Point<int16_t>(50, 172), "a ball to the left starts 50 px left, 28 up");
		require(start({ 100, 200 }, false) == Point<int16_t>(150, 172), "and to the right, 50 px right");

		require(needs_ahead(2001005) && needs_ahead(2121003), "Energy Bolt and Fire Demon need their mob ahead");
		require(!needs_ahead(2221006) && !needs_ahead(2201005) && !needs_ahead(2321001),
				"Chain Lightning, Thunder Bolt and Big Bang don't");

		require(ahead(150, { 160, 220, 100, 200 }, false), "a mob past the start is ahead");
		require(!ahead(150, { 150, 220, 100, 200 }, false), "one that reaches back to it isn't");
		require(ahead(50, { -20, 40, 100, 200 }, true), "on the left, one whose right is short of it is");
		require(!ahead(50, { -20, 50, 100, 200 }, true), "and one that reaches it isn't");
		require(!ahead(150, { 160, 160, 100, 200 }, false), "nor is one with no body");

		require(aim({ 150, 172 }, { 200, 260, 150, 200 }, false) == Point<int16_t>(210, 172),
				"a ball aims 10 px inside a mob's near side");
		require(aim({ 150, 172 }, { 200, 210, 150, 200 }, false).x() == 205, "or at the middle of a narrow one");
		require(aim({ 50, 172 }, { -60, 0, 150, 200 }, true).x() == -10, "from the right side of one on the left");
		require(aim({ 150, 172 }, { 200, 260, 180, 220 }, false).y() == 180, "it comes down to the top of a low mob");
		require(aim({ 150, 172 }, { 200, 260, 100, 160 }, false).y() == 160, "and up to the bottom of a high one");

		require(end({ 0, 0 }, { 300, 400 }) == Point<int16_t>(300, 400), "a ball flies to where it aims");
		require(end({ 0, 0 }, { 800, 0 }) == Point<int16_t>(600, 0), "but no further than 600 px");
		require(end({ 0, 0 }, { -1000, 0 }) == Point<int16_t>(-600, 0), "either way");
		require(end({ 0, 0 }, { 700, 1 }) == Point<int16_t>(599, 0), "cut down to whole pixels");

		require(hit_delay(2201005, 3, false, { 0, 0 }, { 300, 400 }) == 150, "Thunder Bolt hits a mob every 50 ms");
		require(hit_delay(2311004, 3, false, { 0, 0 }, { 300, 400 }) == 0, "a spell with no ball hits at once");
		require(hit_delay(2001005, 0, true, { 0, 0 }, { 300, 400 }) == 750, "a ball's hits land 1.5 ms per pixel");
		require(hit_delay(2001005, 0, true, { 0, 0 }, { 1000, 0 }) == 1500, "all the way to where it aims");

		require(attack_delay(2301002, 400) == 0 && attack_delay(2001005, 400) == 400,
				"Heal hits at once, others at the attack's hit frame");
		require(shown_hits(2301002, 3) == 1 && shown_hits(2001005, 3) == 3, "Heal shows one hit");
		require(across_time(50, 250) == 300 && across_time(-50, -250) == 300, "a ball flies across 1.5 ms per pixel");

		std::cout << "Remote spell passed\n";
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	return 0;
}
