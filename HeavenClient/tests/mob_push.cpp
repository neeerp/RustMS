#include "../Gameplay/Combat/MobPush.h"

#include <cmath>
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

using namespace ms::mob_push;
namespace dash = ms::attack_dash;

static Throw start(Kind kind, int32_t x, int32_t target, bool left)
{
	return Throw{ target, x - target, start_speed(kind, x, target, left) };
}

// The x a mob passes each step, until it stops or `steps` run out.
static std::vector<int32_t> play(Throw course, int32_t x, bool flying, int steps)
{
	std::vector<int32_t> path;

	for (int i = 0; i < steps; i++)
	{
		course.step(x, flying);
		x += static_cast<int32_t>(std::lround(course.distance));
		path.push_back(x);
	}

	return path;
}

static void kinds()
{
	require(kind(dash::RUSH_HERO) == Kind::AHEAD, "Rush");
	require(kind(dash::RUSH_PALADIN) == Kind::AHEAD, "Rush, Paladin");
	require(kind(dash::RUSH_DK) == Kind::AHEAD, "Rush, Dark Knight");
	require(kind(dash::FINAL_CHARGE) == Kind::AHEAD, "Final Charge");
	require(kind(dash::BACKSPIN_BLOW) == Kind::BEHIND, "Backspin Blow");
	require(kind(dash::CORKSCREW_BLOW) == Kind::DRIVEN, "Corkscrew Blow");
	require(kind(dash::CORKSCREW_BLOW_TB) == Kind::DRIVEN, "Thunder Breaker's");
	require(kind(dash::BARRAGE) == Kind::NONE, "Barrage is another kind of throw");
	require(kind(1001004) == Kind::NONE, "a plain attack");
}

static void offsets()
{
	// Corkscrew Blow's rect runs from -140 to 0 and it reaches past that.
	require(offset(dash::CORKSCREW_BLOW, -140, 0, 40, 5, false) == 220, "ahead of a player facing right");
	require(offset(dash::CORKSCREW_BLOW, -140, 0, 40, 5, true) == -220, "ahead of a player facing left");
	require(offset(dash::RUSH_HERO, -250, 0, 60, 0, false) == 345, "Rush");

	// Backspin Blow's runs from 0 to 140 and hits behind the player.
	require(offset(dash::BACKSPIN_BLOW, 0, 140, 40, 5, false) == -200, "behind a player facing right");
	require(offset(dash::BACKSPIN_BLOW, 0, 140, 40, 5, true) == 200, "behind a player facing left");

	// Final Charge rolls nothing and has no margin.
	require(offset(dash::FINAL_CHARGE, -200, 0, 40, 19, false) == 240, "Final Charge");
}

static void speeds()
{
	require(start_speed(Kind::AHEAD, 0, 100, false) == 700.0, "far from the spot");
	require(start_speed(Kind::AHEAD, 1, 100, false) == 300.0, "near the spot");
	require(start_speed(Kind::AHEAD, 0, -100, true) == -700.0, "to the left");
	require(start_speed(Kind::DRIVEN, 99, 100, false) == 700.0, "Corkscrew Blow is always fast");
	require(start_speed(Kind::BEHIND, 0, -100, false) == -700.0, "behind a player facing right");
	require(start_speed(Kind::BEHIND, 0, 50, true) == 300.0, "behind a player facing left, near");
}

static void steps()
{
	// 21 px a step until it is within 100 of the spot, which it reaches in one step.
	require(play(start(Kind::AHEAD, 0, 150, false), 0, false, 5) == std::vector<int32_t>({ 21, 42, 63, 150, 150 }),
			"a mob thrown on the ground");

	require(play(start(Kind::BEHIND, 0, -150, false), 0, false, 5)
				== std::vector<int32_t>({ -21, -42, -63, -150, -150 }), "thrown the other way");

	// Already close, it stops.
	require(play(start(Kind::AHEAD, 95, 100, false), 95, false, 2) == std::vector<int32_t>({ 95, 95 }), "at the spot");

	// Past the spot it goes on the way the attacker faces, then stops once it crosses.
	Throw crossed = start(Kind::AHEAD, 50, 0, true);
	crossed.step(-1, false);
	require(crossed.speed == 0.0 && crossed.distance == 0.0, "stopped having crossed the spot");

	// A flier only slows.
	Throw flier = start(Kind::AHEAD, 0, 150, false);
	flier.step(100, true);
	require(flier.speed == 700.0 / 3, "a flier slows near the spot");
	require(std::abs(flier.distance - 7.0) < 0.001, "at a third of the speed");
	flier.step(149, true);
	require(std::abs(flier.speed - 700.0 / 9) < 0.001, "and slows again");
}

int main()
{
	kinds();
	offsets();
	speeds();
	steps();

	std::cout << "ok\n";
}
