#include "../Gameplay/Combat/ShotRain.h"

#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool same(ms::Rectangle<int16_t> a, ms::Rectangle<int16_t> b)
{
	return a.left() == b.left() && a.right() == b.right() && a.top() == b.top() && a.bottom() == b.bottom();
}

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::shot_rain;

		require(kind(3111004) == Kind::OVER_USER && kind(13111000) == Kind::OVER_USER, "Arrow Rain falls over the user");
		require(kind(3111003) == Kind::OVER_HIT && kind(3211003) == Kind::OVER_HIT, "Inferno and Blizzard where they hit");
		require(kind(3211004) == Kind::OVER_MOBS, "3211004 on each mob");
		require(kind(3101005) == Kind::NONE && kind(0) == Kind::NONE, "other shots rain nothing");
		require(burns(3111003) && !burns(3211003), "only Inferno sets the ground on fire");

		Rectangle<int16_t> rain{ -350, 100, -50, 50 };

		require(same(over_user(rain, { 1000, 200 }, true), { 650, 1100, -100, 0 }),
				"Arrow Rain's area as it is facing left, 250 px up");
		require(same(over_user(rain, { 1000, 200 }, false), { 900, 1350, -100, 0 }), "mirrored facing right");

		Rectangle<int16_t> fire{ -150, 150, -100, 100 };

		require(same(around(fire, { 500, 300 }), { 350, 650, 200, 400 }), "Inferno's around the hit, never mirrored");
		require(same(raised({ 350, 650, 200, 400 }), { 350, 650, 150, 230 }), "its rain from higher up");

		require(same(over_mob({ 100, 160, 50, 120 }), { 55, 205, 110, 120 }), "a narrow mob's strip is 150 px");
		require(same(over_mob({ 100, 161, 50, 120 }), { 55, 206, 110, 120 }), "an odd width keeps its odd pixel");
		require(same(over_mob({ 100, 300, 50, 120 }), { 100, 300, 110, 120 }), "a wide mob's is its own width");

		Params arrows{ 85, 230, 8, 150, 90, 0, 500 };
		Schedule schedule(arrows, 1000);

		require(!schedule.drop(999), "nothing before the rain starts");
		require(schedule.drop(1000), "the first arrows drop at the hit");
		require(!schedule.drop(1089) && schedule.drop(1090), "the next an interval on");
		require(schedule.drop(1300), "a late update drops one wave");
		require(schedule.drop(1300) && !schedule.drop(1300), "and catches up a wave an update");
		require(schedule.drop(1360) && !schedule.over(1500) && schedule.over(1501), "and it's over after its duration");
		require(!schedule.drop(1600), "nothing drops after");

		Params flames{ 105, 130, 5, 400, 90, 150, 500 };
		Schedule later(flames, 1000);

		require(!later.drop(1149) && later.drop(1150), "Inferno's flames start 150 ms after the hit");

		require(fall(arrows, true, 20) == Point<int16_t>(-85, 230), "arrows slant the way the user faces");
		require(fall(arrows, false, 20) == Point<int16_t>(85, 230), "either way");
		require(fall(arrows, false, 0) == Point<int16_t>(85, 237) && fall(arrows, false, 40) == Point<int16_t>(85, 223),
				"and fall up to 20 x / y further or shorter");

		Params up{ 0, -150, 3, 150, 90, 0, 500 };

		require(fall(up, true, 0) == Point<int16_t>(0, -150), "straight rains don't vary");
		require(flight(arrows, 0) == 0 && flight(arrows, 299) == 299 && flight(flames, 0) == 250,
				"each falls fall - 150 ms, plus up to 299");
		require(std::abs(opacity(0) - 128 / 255.0f) < 1e-6f && std::abs(opacity(63) - 191 / 255.0f) < 1e-6f,
				"and fades to half to three quarters opaque");

		std::vector<int16_t> xs = columns({ 0, 60, 0, 10 }, 20);

		require(xs.size() == 3 && xs[0] == 0 && xs[1] == 20 && xs[2] == 40, "flames every 20 px, not on the right edge");
		require(columns({ 0, 60, 0, 10 }, 0).empty(), "none without a distance");

		Rectangle<int16_t> ground_area{ 0, 200, 100, 200 };

		require(ground({ 0, 150, 100, 150 }, 50, ground_area) == std::optional<int16_t>(150), "on a level platform");
		require(ground({ 0, 100, 100, 200 }, 50, ground_area) == std::optional<int16_t>(150), "on a slope");
		require(!ground({ 0, 250, 100, 250 }, 50, ground_area), "not below the area");
		require(!ground({ 0, 50, 100, 50 }, 50, ground_area), "nor above it");
		require(!ground({ 60, 150, 100, 150 }, 50, ground_area), "nor off the platform's ends");
		require(!ground({ 50, 150, 50, 200 }, 50, ground_area) && !ground({ 150, 150, 0, 150 }, 50, ground_area),
				"nor on walls and right-to-left platforms");
		require(ground({ 0, 150, 100, 150 }, 100, ground_area) == std::optional<int16_t>(150), "its ends count");

		require(flame_y(150, 0) == 148 && flame_y(150, 4) == 152, "flames sit within 2 px of the ground");
		require(flame_fade(0) == 100 && flame_fade(4) == 500, "and fade in over 100 to 500 ms");

		require(fade(1.0f, 0.5f, 100, 200, 50) == 1.0f && fade(1.0f, 0.5f, 100, 200, 250) == 0.5f, "fades hold at their ends");
		require(std::abs(fade(1.0f, 0.5f, 100, 200, 150) - 0.75f) < 1e-6f, "and move evenly between");

		std::cout << "shot rain tests passed\n";
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << '\n';

		return 1;
	}

	return 0;
}
