#include "../Gameplay/MapleMap/SummonRules.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

using namespace ms::summon_rules;
using ms::Point;

int main()
{
	try
	{
		require(is_summon(3111005) && is_summon(1321007) && !is_summon(3111004), "summon skills are known");
		require(spreads(GAVIOTA) && !spreads(3111005), "Gaviota spreads its hits");
		require(!can_act(STATIONARY, 3111002), "a Puppet only stands there");
		require(can_act(STATIONARY, OCTOPUS), "the Octopus stays put but shoots");
		require(attack_interval(3111005) == 3000 && attack_interval(GAVIOTA) == 1500,
				"Gaviota and Wrath of the Octopi attack twice as often");
		require(attacks_riding(0, 3111005) && !attacks_riding(1902000, 3111005) && !attacks_riding(1932000, 2121005),
				"a rider's summons stand idle");
		require(attacks_riding(1902000, OCTOPUS) && attacks_riding(1932000, GAVIOTA),
				"but the Octopi and Gaviota still attack");
		require(pick(-1, 1 << 1 | 1) == MOVE && pick(-1, 6 << 1) == FLY && pick(-1, 0) == STAND,
				"the move action picks move, fly, or stand");
		require(pick(ATTACK1, 1 << 1) == ATTACK1, "a one-shot plays over the move action");

		// A follower glides 7 px a step until 70 px from its owner.
		Follower elquines{ 0, 0, false };
		follow(elquines, 100, 0, false, 70, TICK);
		require(elquines.x == 7 && elquines.moving && !elquines.left, "it glides 7 px a step toward the owner");

		for (int i = 0; i < 10; i++)
			follow(elquines, 100, 0, false, 70, TICK);

		require(elquines.x == 30 && !elquines.moving, "it stops 70 px short");
		follow(elquines, 100, 0, true, 70, TICK);
		follow(elquines, -100, 0, false, 70, TICK);
		require(elquines.left && elquines.x == 23, "it turns to an owner behind it");

		// It climbs to an owner 30 px up only after 30 steps apart, at once
		// to one more than 100 px away.
		Follower soul{ 0, 0, false };

		for (int i = 0; i < 29; i++)
			follow(soul, 0, -30, false, 42, TICK);

		require(soul.y == 0, "an owner a little above is waited for");
		follow(soul, 0, -30, false, 42, TICK);
		require(soul.y == -7, "after 30 steps it climbs");

		for (int i = 0; i < 4; i++)
			follow(soul, 0, -30, false, 42, TICK);

		require(soul.y == -30 && !soul.climbing, "it stops at the owner's height");
		follow(soul, 0, 90, false, 42, TICK);
		require(soul.y == -23, "an owner far below is gone after at once");

		require(!lost(300, -200) && lost(301, 0) && lost(0, 201), "an owner beyond 300 x 200 px is lost");

		// A flier picks a spot near the owner and flies there.
		int32_t calls = 0;
		auto middle = [&](int32_t n) { calls++; return n / 2; };
		Circler hawk{ 0, 0, false };
		circle(hawk, 500, 100, false, middle, TICK);
		require(hawk.tx == 500 && hawk.ty == 100 && hawk.retarget == 90 * TICK, "the spot is around the owner");
		require(std::abs(hawk.x - FLY_SPEED * TICK * 500 / std::hypot(500, 100)) < 1e-9 && !hawk.left,
				"it flies toward the spot");
		circle(hawk, 500, 100, false, middle, TICK);
		require(calls == 3, "it keeps its spot");
		auto low = [](int32_t) { return 0; };
		Circler eagle{ 0, 0, false };
		circle(eagle, 0, 0, true, low, TICK);
		require(eagle.tx == -400 && eagle.ty == -150 && eagle.left, "a stationary flier ranges over its spawn");

		// Puppets go 200 px ahead onto a foothold within 80 px, else nearer.
		auto flat = [](int16_t, int16_t) -> int16_t { return 100; };
		auto placed = place(3111002, 0, 100, false, -1000, 1000, flat);
		require(placed && placed->first == 200 && placed->second == 99, "a Puppet stands 200 px ahead");
		placed = place(3111002, 0, 100, true, -150, 1000, flat);
		require(placed && placed->first == -150, "it stays inside the map");
		auto ledge = [](int16_t x, int16_t) -> int16_t { return x < 50 ? 100 : 500; };
		placed = place(3111002, 0, 100, false, -1000, 1000, ledge);
		require(placed && placed->first == 35, "past a ledge it steps back 15 px at a time");
		auto pit = [](int16_t, int16_t) -> int16_t { return 900; };
		require(!place(3111002, 0, 100, false, -1000, 1000, pit), "with no foothold it isn't cast");
		placed = place(3111005, 10, 20, true, -1000, 1000, pit);
		require(placed && placed->first == 10 && placed->second == 20, "a hawk starts at its owner");
		placed = place(OCTOPUS, 0, 100, true, -1000, 1000, flat);
		require(placed && placed->first == -45, "the Octopus sits 45 px ahead");

		require(physical_damage(100, 40, 100, 0.0) == 215 && physical_damage(100, 40, 100, 0.999999) == 289,
				"a hit is 70-100% of the main stat times 2.5, plus the others");
		require(physical_damage(100, 40, 50, 0.0) == 107, "times the summon's pad percent");
		require(touch_damage(300, 20, 0.5) == 189 && touch_damage(300, 20, 0.999999) == 339 &&
				touch_damage(12, 20, 0.5) == 0, "a Puppet takes the square of the mob's attack, scaled down");
		require(main_stat(311) == DEX && main_stat(412) == LUK && main_stat(212) == INT && main_stat(1100) == STR &&
				main_stat(520) == DEX, "each job has its main stat");

		// A Beholder heals a hurt owner, but not out of Berserk.
		require(!aura_heals(50, 50, 40, 0) && aura_heals(10, 50, 40, 0), "the Aura heals a hurt owner");
		require(aura_heals(400, 1000, 40, 50) && !aura_heals(480, 1000, 40, 50) && aura_heals(600, 1000, 40, 50),
				"it keeps a berserk owner berserk");

		// Hex picks among its stats, the later ones likelier.
		std::array<int32_t, 5> none{};
		require(hex_pick({ 20, 0, 0, 0, 0 }, none, 12345) == HEX_PDD, "level 1 raises weapon defense");
		std::array<int32_t, 5> all{ 100, 100, 25, 50, 15 };
		require(hex_pick(all, none, 0) == HEX_PDD && hex_pick(all, none, 1) == HEX_MDD &&
				hex_pick(all, none, 5) == HEX_ACC && hex_pick(all, none, 6) == HEX_EVA &&
				hex_pick(all, none, 14) == HEX_PAD && hex_pick(all, none, 15) == HEX_PDD,
				"one in 15 weapon defense, five in 15 weapon attack");
		require(hex_pick(all, { 0, 0, 0, 0, 15 }, 14) == HEX_EVA, "a stat the owner has is passed over for a lower one");
		require(hex_pick(all, { 0, 100, 25, 50, 15 }, 3) == HEX_PDD, "down to the lowest they lack");
		require(hex_pick(all, { 100, 100, 25, 50, 15 }, 3) == HEX_ACC, "or the one picked, with all");
		require(hex_pick(none, none, 3) == -1, "no Hex, no buff");

		// The hits land after attackAfter: the spreading kinds' 120 ms
		// later together, the others' 50 ms apart.
		require(hit_after(GAVIOTA, 0) == 120 && hit_after(GAVIOTA, 3) == 120, "a spread lands at once");
		require(hit_after(3111005, 0) == 0 && hit_after(2121005, 2) == 100, "the others 50 ms apart");
		require(is_octopus(WRATH_OF_THE_OCTOPI) && !is_octopus(GAVIOTA), "only the Octopi fire a ball");

		// The ball leaves the Octopus's sp, mirrored when it faces right,
		// and flies 600 ms per bulletSpeed px.
		Point<int16_t> sp(-23, -18);
		require(muzzle({ 100, 50 }, sp, true) == Point<int16_t>(77, 32), "facing left, sp as drawn");
		require(muzzle({ 100, 50 }, sp, false) == Point<int16_t>(123, 32), "facing right, mirrored");
		require(ball_time({ 0, 0 }, { 300, 0 }, 300) == 600 && ball_time({ 0, 0 }, { 30, 40 }, 300) == 100,
				"300 px at bulletSpeed 300 takes 600 ms");
		require(ball_time({ 0, 0 }, { 0, 1 }, 300) == 2, "cut to a whole ms");
		require(ball_angle({ 100, 0 }, { 0, 0 }) == 0.0f && ball_angle({ 0, 0 }, { 100, 0 }) == 0.0f,
				"level either way: mirrored, not turned");
		require(ball_angle({ 100, 0 }, { 0, 100 }) < -0.78f && ball_angle({ 0, 0 }, { 100, 100 }) > 0.78f,
				"turned down toward a mob below");

		std::cout << "Summon rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
