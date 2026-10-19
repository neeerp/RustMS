#include "../Gameplay/WeatherRules.h"
#include <cmath>
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

// Hands out the given numbers in order, as rand() would.
struct Rolls
{
	std::vector<int32_t> values;
	size_t at = 0;

	int32_t operator()()
	{
		if (at >= values.size()) throw std::runtime_error("too many rolls");

		return values[at++];
	}
};

int main()
{
	try
	{
		using namespace ms::weather;

		require(is_weather(5120000) && is_weather(5122000) && !is_weather(5100000) && !is_weather(0), "only 512xxxx");

		// FloatNotice/0 is 12 | 4 | 12: a 30 px message takes 8 tiles.
		Bar plain = bar(800, 12, 4, 12, 30);
		require(plain.tiles == 8 && plain.width == 56 && plain.left == 372, "tiles to cover the text, centred");
		require(plain.text_x == 13, "the text centred over the tiles");
		Bar wide = bar(800, 9, 782, 9, 200);
		require(wide.tiles == 1 && wide.width == 800 && wide.left == 0, "FloatNotice/8 spans the screen");
		require(bar(800, 55, 722, 55, 900).left == -377, "truncates toward zero");
		require(bar(800, 12, 4, 12, 0).tiles == 0, "no text, no tiles");

		require(particle_count(true, 100, 0) == 30 && particle_count(true, 100, 24) == 54, "30 to 54 %");
		require(particle_count(true, 300, 49) == 162, "flowerage's 300");
		require(particle_count(false, 35, 7) == 35 && particle_count(false, 0, 7) == 0, "all of a fixed count");

		// Snow: type 1, falling, 2 canvases.
		Rolls snow{ { 3, 2, 45, 1500, 7, 500, 200, 150 } };
		Particle flake = particle(1, 0, 0, 2, 0, std::ref(snow));
		require(flake.size == 21 && flake.phase == 45 && flake.fade == 4500, "size, phase and fade first");
		require(flake.frame == 1, "then the canvas");
		require(flake.sx == 100 && flake.sy == -100 && flake.ex == 150 && flake.ey == 200, "down 300, drifting");
		require(flake.spin == 0 && flake.period == 50000 / 21, "no spin; 50 s over its size");
		require(snow.at == snow.values.size(), "every roll used");

		// A spinning flower blowing left at speed 1.
		Rolls flower{ { 0, 0, 0, 0, 0, 100, 50, 30, 700 } };
		Particle petal = particle(2, 2, 1, 1, 0, std::ref(flower));
		require(petal.size == 15 && petal.sx == 300 && petal.ex == -100 && petal.ey == -250 - 70, "left 400");
		require(petal.spin == 1000 && petal.period == 42000 / 15, "spins; faster at speed 1");

		// An animated water drop: the animation is drawn after the spot, not before.
		Rolls water{ { 1, 1, 10, 0, 400, 300, 100, 2 } };
		Particle drop = particle(1, 0, 0, 0, 3, std::ref(water));
		require(drop.frame == 2 && drop.sx == 0 && drop.sy == 0, "the animation after the spot");

		// Direction 5 bursts from the centre, the angle in radians, the time drawn last.
		Rolls burst{ { 0, 0, 1, 0, 0, 5, 5, 15000 } };
		Particle spark = particle(1, 5, 0, 1, 0, std::ref(burst));
		require(spark.sx == 395 && spark.sy == 295, "from the centre");
		require(spark.ex == static_cast<int32_t>(std::cos(1.0) * 500.0 + 400.0), "cos of 1 radian");
		require(spark.ey == static_cast<int32_t>(std::sin(1.0) * 500.0 + 300.0), "sin of 1 radian");
		require(spark.period == 50000 / 15, "rand % 30000 - 15000 around the base");

		Rolls fountain{ { 0, 0, 2, 0, 0, 0, 0, 0 } };
		require(particle(1, 4, 0, 1, 0, std::ref(fountain)).ey == 0, "direction 4's truncated sine");

		Rolls still{ { 0, 0, 0, 0, 0, 10, 20 } };
		Particle fixed = particle(1, 6, 0, 1, 0, std::ref(still));
		require(fixed.ex == fixed.sx && fixed.period == 0, "direction 6 stays");

		require(moved(300, 0, 1000) == 0 && moved(300, 500, 1000) == 150 && moved(300, 2500, 1000) == 750, "linear");
		require(moved(-100, 999, 1000) == -99, "truncated toward zero");
		require(moved(300, 500, 0) == 0, "no period, no movement");

		require(wrap(5, 0, 10) == 5 && wrap(12, 0, 10) == 2 && wrap(-1, 0, 10) == 9, "positive modulo");
		require(wrap(3, 7, 0) == 7, "an empty window");

		// A flake at the view's centre doesn't sway at its start or after a full turn.
		Particle still_flake{ 20, 90, 3000, 0, 0, 0, 0, 0, 0, 0 };
		Spot centre = world_spot(still_flake, 0, 0, 0, 100, 800, 600);
		require(centre.x == 0 && centre.y == 0, "no sway at first");
		Spot turned = world_spot(still_flake, 2000, 0, 0, 100, 800, 600);
		require(turned.x == 0 && turned.y == 0, "a full turn after 2 s");
		Spot half = world_spot(still_flake, 1000, 0, 0, 100, 800, 600);
		require(half.x == 0 && half.y == -1, "half a turn from 90 degrees: 20 px up at 5 %");
		Particle east{ 20, 0, 3000, 0, 0, 0, 0, 0, 0, 0 };
		require(world_spot(east, 1000, 0, 0, 100, 800, 600).x == -20, "the whole size across at 100 %");
		require(world_spot(east, 1000, 0, 0, 50, 800, 600).x == -10, "x scales the sway");

		// The view moved 1000 px right: the flake wraps into it.
		Spot wrapped = world_spot(still_flake, 0, 1000, 0, 100, 800, 600);
		require(wrapped.x == 820 && wrapped.y == 0, "wrapped around the view");
		Particle low{ 15, 0, 3000, 0, 0, 290, 0, 590, 0, 1000 };
		require(world_spot(low, 500, 0, 0, 0, 800, 600).y == -175, "out the bottom, in at the top");

		Particle fan{ 15, 0, 3000, 0, 400, 300, 800, 600, 0, 1000 };
		Spot fan_half = screen_spot(fan, 500);
		require(fan_half.x == 600 && fan_half.y == 450, "toward the target");
		Spot fan_end = screen_spot(fan, 1000);
		require(fan_end.x == 400 && fan_end.y == 300, "back to the start");
		Particle back{ 15, 0, 3000, 0, 400, 300, 0, 0, 0, 1000 };
		Spot back_end = screen_spot(back, 999);
		require(back_end.x == 1 && back_end.y == 1, "the box excludes the far end");

		Particle spinner{ 15, 0, 3000, 0, 0, 0, 0, 0, 1000, 0 };
		require(spin_angle(spinner, 250) == 90.0 && spin_angle(spinner, 1250) == 90.0, "one turn per spin");
		require(spin_angle(still_flake, 250) == 0.0, "type 1 doesn't spin");

		Particle faded{ 20, 0, 4000, 0, 0, 0, 0, 0, 0, 0 };
		require(fade_in(faded, 0, false) == 1.0f && fade_in(faded, 4000, false) == 215.0f, "1 to 255 - 2 size");
		require(fade_in(faded, 2000, false) == 108.0f && fade_in(faded, 9000, false) == 215.0f, "linear, then held");
		require(fade_in(faded, 0, true) == 255.0f, "animations at once");

		require(fade_out_time(0) == 1000 && fade_out_time(1999) == 1999, "1000 to 1999 ms");
		require(fade_out(200.0f, 0, 1000) == 200.0f && fade_out(200.0f, 500, 1000) == 100.0f, "from where it was");
		require(fade_out(200.0f, 1000, 1000) == 0.0f, "to nothing");

		Spread spread_state;
		std::pair<int32_t, int32_t> first = spread(spread_state);
		std::pair<int32_t, int32_t> second = spread(spread_state);
		require(first.first == 134 && first.second == 100, "a sixth of the screen first");
		require(second.first == 301 && second.second == 225, "then wider");
		require(burst_offset(134, 0) == -67 && burst_offset(134, 133) == 66, "centred on the screen");

		require(BURSTS[1].first == 1000 && BURSTS[1].second == 5, "squib1: five a second");
		require(FIREWORK_X[0] == -230 && FIREWORK_X[2] == 230 && FIREWORK_Y == -60, "three across");

		std::cout << "Weather rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
