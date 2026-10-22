#include "../Gameplay/MapleMap/KiteRules.h"
#include <cmath>
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
		using namespace ms::kite;

		// The Free Market's rooms (910xxxxxx) refuse kites; the rest of the world, Henesys's market included, doesn't.
		require(!allowed_on(910000000), "free market entrance");
		require(!allowed_on(910000022), "free market room");
		require(allowed_on(100000100), "henesys market");
		require(allowed_on(103000000), "kerning city");
		require(allowed_on(920000000), "orbis tower");

		// Always three lines, empty ones kept.
		require(join({ "a", "b", "c" }) == "a\nb\nc", "three lines");
		require(join({ "", "", "" }) == "\n\n", "empty lines");
		require(join({ "hi" }) == "hi\n\n", "missing lines");

		// The viewer's rows: the remainder after two breaks is the third.
		auto rows = split("a\nb\nc");
		require(rows.size() == 3 && rows[0] == "a" && rows[1] == "b" && rows[2] == "c", "split three");
		rows = split("\n\n");
		require(rows.size() == 3 && rows[0].empty() && rows[2].empty(), "split empty");
		rows = split("one");
		require(rows.size() == 1 && rows[0] == "one", "split one");
		rows = split("a\nb\nc\nd");
		require(rows.size() == 3 && rows[2] == "c", "split stops at three");

		// The box is 60x80 around (x-3, y-100).
		require(hit(0, 0, -3, -100), "centre");
		require(hit(0, 0, -33, -140), "top left");
		require(hit(0, 0, 27, -60), "bottom right");
		require(!hit(0, 0, -34, -100), "left of it");
		require(!hit(0, 0, 28, -100), "right of it");
		require(!hit(0, 0, -3, -141), "above it");
		require(!hit(0, 0, -3, -59), "below it");

		// 3 px either way, once every 2 s of 30 ms steps.
		require(std::abs(bob(0.0)) < 1e-9, "bob starts level");
		require(std::abs(bob(M_PI / 2) - 3.0) < 1e-9, "bob top");
		double period = 2 * M_PI / BOB_STEP * BOB_MS;
		require(std::abs(period - 2000.0) < 1.0, "bob period");

		// The fade runs a second.
		require(fade_opacity(FADE) == 1.0f, "fade start");
		require(fade_opacity(500) == 0.5f, "fade half");
		require(fade_opacity(0) == 0.0f, "fade end");

		require(hover_text("Lanec", "is wishing you a happy new year.") == "Laneci" "s wishing you a happy new year.",
				"hover text runs together");

		std::cout << "kite_rules: ok\n";
		return 0;
	}
	catch (const std::exception &e)
	{
		std::cerr << "kite_rules: " << e.what() << '\n';
		return 1;
	}
}
