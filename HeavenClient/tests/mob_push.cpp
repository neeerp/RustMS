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

static void spins()
{
	require(kind(NINJA_STORM) == Kind::SPUN, "Ninja Storm");
	require(kind(ROLLING_SPIN) == Kind::SPUN, "Rolling Spin");
	require(kind(FINAL_TOSS) == Kind::TOSSED, "Final Toss");

	// Range 110, a roll of 5, a mob 40 wide.
	require(spun_offset(110, 40, 5, false) == 145, "spun to the right");
	require(spun_offset(110, 40, 5, true) == -145, "spun to the left");

	require(start_speed(Kind::SPUN, 0, 145, false) == 300.0, "toward the spot on the right");
	require(start_speed(Kind::SPUN, 145, 145, false) == 300.0, "on the spot, to the right");
	require(start_speed(Kind::SPUN, 200, 145, false) == 0.0, "past the spot on the right");
	require(start_speed(Kind::SPUN, 0, -145, true) == -300.0, "toward the spot on the left");
	require(start_speed(Kind::SPUN, -145, -145, true) == -300.0, "on the spot, to the left");
	require(start_speed(Kind::SPUN, -200, -145, true) == 0.0, "past the spot on the left");

	// 9 px a step until it is within 100 of the spot.
	require(play(start(Kind::SPUN, 0, 120, false), 0, false, 5) == std::vector<int32_t>({ 9, 18, 27, 120, 120 }),
			"a spun mob on the ground");

	// Past the spot it stays, however near.
	require(play(start(Kind::SPUN, 170, 145, false), 170, false, 3) == std::vector<int32_t>({ 170, 170, 170 }),
			"a mob past the spot");

	// A flier only slows within 40.
	Throw flier = start(Kind::SPUN, 0, 120, false);
	flier.air_near = SPIN_NEAR;
	flier.step(50, true);
	require(flier.speed == 300.0, "a spun flier 70 off keeps its speed");
	flier.step(90, true);
	require(flier.speed == 100.0, "and slows within 40");
}

static void moved()
{
	require(moves(Kind::AHEAD, true, 0, false, true), "Rush moves anything");

	require(moves(Kind::SPUN, false, 1, false, false), "a spin moves what it hits");
	require(!moves(Kind::SPUN, true, 1, true, false), "not a boss");
	require(!moves(Kind::SPUN, false, 0, true, false), "not on a miss");

	require(moves(Kind::TOSSED, false, 1, true, false), "a toss lifts a walker");
	require(!moves(Kind::TOSSED, true, 1, true, false), "not a boss");
	require(!moves(Kind::TOSSED, false, 0, true, false), "not on a miss");
	require(!moves(Kind::TOSSED, false, 1, false, false), "not a flier or one that stays put");
	require(!moves(Kind::TOSSED, false, 1, true, true), "not under water");
}

static void barrage()
{
	require(kind(SNATCH) == Kind::PULLED, "Snatch pulls");
	require(kind(BARRAGE) == Kind::NUDGED && kind(BARRAGE_TB) == Kind::NUDGED, "Barrage nudges");
	require(moves(Kind::PULLED, true, 0, false, false), "whatever it hits");
	require(moves(Kind::NUDGED, true, 0, false, false), "likewise");

	require(nudge(0, false) == 17 && nudge(1, false) == 6 && nudge(4, false) == 6, "the first hit goes furthest but the last");
	require(nudge(5, false) == 27 && nudge(5, true) == -27, "the last, the way the attacker faces");
	require(nudge(6, false) == 0, "and nothing past the table");

	Throw course{ 500, 0, 0.0 };
	course.nudged = true;
	course.nudge = nudge(0, true);
	require(course.take_nudge() == -17, "all at once");
	require(course.take_nudge() == 0, "and no more");
}

static void again()
{
	require(flinches_again(0, 2112), "a Lv 4 Aran");
	require(flinches_again(0, 2000), "a Legend");
	require(flinches_again(BARRAGE, 512), "Barrage");
	require(flinches_again(BARRAGE_TB, 1511), "a Thunder Breaker's");
	require(flinches_again(VAMPIRE, 1410), "Vampire");
	require(!flinches_again(0, 112), "a Hero doesn't");
	require(!flinches_again(5121001, 512), "nor another Buccaneer skill");
}

int main()
{
	kinds();
	offsets();
	speeds();
	steps();
	spins();
	moved();
	barrage();
	again();

	std::cout << "ok\n";
}
