#include "../Gameplay/Combat/AranCombo.h"
#include "../Gameplay/Combat/CombatStep.h"

#include <algorithm>
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::aran_combo;

		require(ability(2000) == 20000017 && ability(2100) == 21000000 && ability(2112) == 21000000,
				"a beginner's Combo Ability is its own");

		require(needs(21100004) == 30 && needs(21100005) == 30, "Combo Smash and Combo Drain need 30");
		require(needs(21110004) == 100, "Combo Penril 100");
		require(needs(21120006) == 200 && needs(21120007) == 200, "Combo Tempest and Combo Barrier 200");
		require(needs(21100002) == 0 && needs(0) == 0, "the rest nothing");

		require(counts(true, 1, 1, POLEARM), "a polearm hit of the player's counts");
		require(!counts(false, 1, 1, POLEARM), "not someone else's");
		require(!counts(true, 0, 1, POLEARM), "not a miss");
		require(!counts(true, 1, 0, POLEARM), "not without Combo Ability");
		require(!counts(true, 1, 1, 143), "not with a spear");

		require(!idle(5, 1000, 4000) && idle(5, 1000, 4001), "a count goes once 3 seconds have passed since it came");
		require(!idle(0, 1000, 9000), "and none has nothing to go");

		require(tier(1) == 0 && tier(29) == 0 && tier(30) == 1 && tier(99) == 1, "the first tiers break at 30");
		require(tier(100) == 2 && tier(199) == 2 && tier(200) == 3 && tier(30000) == 3, "and 100 and 200");

		require(digits(305) == std::vector<int32_t>({ 5, 0, 3 }), "the digits come the ones first");
		require(digits(0).empty() && digits(123456).size() == 5, "five at most");

		require(digit_y(0, 1) == 165, "a lone digit sits at 165");
		require(digit_y(0, 2) == 169 && digit_y(1, 2) == 165, "the first is always at 165");
		require(digit_y(0, 3) == 165 && digit_y(1, 3) == 169 && digit_y(2, 3) == 165, "and they alternate");

		require(bounce_height(31, 0) == 10 && bounce_height(31, 1) == 4, "the lowest digit bounces 10, the rest 4");
		require(bounce_height(30, 0) == 10 && bounce_height(30, 1) == 10, "and the zeros under it");
		require(bounce_height(100, 2) == 10 && bounce_height(140, 2) == 4, "up to the lowest that isn't 0");
		require(bounce_height(200, 0) == 0 && bounce_height(201, 0) == 0, "nothing bounces from 200");

		require(bounce(10, 0) == 0 && bounce(10, 25) == 5 && bounce(10, 50) == 10, "10 goes down evenly in 50 ms");
		require(bounce(10, 75) == 10 && bounce(10, 175) == 6 && bounce(10, 300) == 0, "and springs back by 300");
		require(bounce(4, 50) == 4 && bounce(4, 90) == 4 && bounce(4, 299) == 0, "4 too");
		require(bounce(0, 25) == 0, "no height, no bounce");

		require(attack(0, 10, 1) == 0 && attack(9, 10, 1) == 0 && attack(57, 10, 1) == 5, "a point for every ten");
		require(attack(300, 10, 1) == 10 && attack(300, 3, 2) == 6, "up to x tens");
		require(critical(2000) == 20000018 && critical(2110) == 21110000, "Combo Critical");
		require(critical(9, 5, 3, 5).rate == 10 && critical(9, 5, 3, 5).damage == 0, "10 percent before the first ten");
		require(critical(57, 5, 3, 5).rate == 25 && critical(57, 5, 3, 5).damage == 25, "y and damage for every ten");
		require(critical(300, 5, 3, 5).rate == 25 && critical(300, 5, 3, 5).damage == 25, "up to x tens");

		require(pushed(100, 10, false) == 10 && pushed(33, 60, false) == 19, "Smart Knockback keeps its percent");
		require(pushed(100, 10, true) == 100 && pushed(100, 0, false) == 100, "not of a boss, nor without it");

		Command thirty = command(30);
		require(thirty.index == 1 && thirty.first == 21100004 && thirty.second == 21100005, "30 frees Combo Smash and Drain");
		Command hundred = command(100);
		require(hundred.index == 2 && hundred.first == 21110004 && hundred.second == 21100005, "100 Combo Penril");
		Command two_hundred = command(200);
		require(two_hundred.index == 3 && two_hundred.first == 21120006 && two_hundred.second == 21120007,
				"200 Combo Tempest and Barrier");
		require(command(31).index == 0 && command(300).index == 0, "only those counts announce anything");

		auto knows = [](std::vector<int32_t> skills)
		{
			return [skills](int32_t skill)
			{
				for (int32_t known : skills)
					if (known == skill)
						return 1;

				return 0;
			};
		};
		auto all = knows({ 21100004, 21100005, 21110004, 21120006, 21120007 });
		auto smash = knows({ 21100004 });

		require(enabled(TO_THE_LEFT, 2110, POLEARM, smash) && enabled(TO_THE_RIGHT, 2112, POLEARM, smash),
				"an Aran with a polearm and Combo Smash");
		require(!enabled(DOWN_TWICE, 2110, POLEARM, smash) && enabled(DOWN_TWICE, 2110, POLEARM, all),
				"down twice needs Combo Drain");
		require(!enabled(TO_THE_LEFT, 2110, 130, all) && !enabled(TO_THE_LEFT, 1100, POLEARM, all),
				"no polearm or no Aran, no command");
		require(enabled(TO_THE_LEFT, 2000, POLEARM, all), "a Legend too");

		auto step = knows({ 21001001 });
		require(enabled(STEP_LEFT, 2100, 130, step) && !enabled(STEP_RIGHT, 2100, 130, smash)
				&& !enabled(STEP_LEFT, 1100, POLEARM, step), "Combat Step with any weapon, for an Aran who knows it");
		require(enabled(DOUBLE_SWING, 2100, POLEARM, knows({ 20000014 }))
				&& enabled(DOUBLE_SWING, 2000, POLEARM, knows({ 21000002 }))
				&& !enabled(DOUBLE_SWING, 2100, 130, knows({ 21000002 })),
				"either Double Swing, with a polearm");
		require(enabled(CHARGE_LEFT, 2110, POLEARM, knows({ 21100002 }))
				&& !enabled(FINAL_TOSS, 2111, POLEARM, knows({ 21100002 })), "each its own skill");
		require(enabled(RAGE, 100, 130, smash) && enabled(RAGE, 2200, 0, smash), "Rage of Pharaoh for anyone");

		require(skill(DOUBLE_SWING, 2000) == 20000014 && skill(DOUBLE_SWING, 2100) == 21000002
				&& skill(TRIPLE_SWING, 2000) == 20000015 && skill(FINAL_BLOW, 2000) == 20000016
				&& skill(FINAL_BLOW, 2112) == 21120005, "the Legend's copies");
		require(skill(RAGE, 100) == 1020 && skill(RAGE, 1100) == 10001020 && skill(RAGE, 2112) == 20001020
				&& skill(RAGE, 2200) == 20011020 && skill(RAGE, 2001) == 20011020,
				"Rage of Pharaoh's copy for each family");

		require(pick(TO_THE_LEFT, 29, all) == 0 && pick(TO_THE_LEFT, 30, all) == 21100004, "Smash from 30");
		require(pick(TO_THE_RIGHT, 150, all) == 21110004 && pick(TO_THE_RIGHT, 200, all) == 21120006,
				"Penril from 100, Tempest from 200");
		require(pick(TO_THE_LEFT, 250, smash) == 21100004, "the best the player knows");
		require(pick(DOWN_TWICE, 30, all) == 21100005 && pick(DOWN_TWICE, 199, all) == 21100005
				&& pick(DOWN_TWICE, 200, all) == 21120007, "Drain from 30, Barrier from 200");
		require(pick(DOWN_TWICE, 250, smash) == 0, "nothing the player doesn't know");

		require(faces_right(TO_THE_RIGHT, false) == true && faces_right(TO_THE_LEFT, false) == false,
				"each turns its way");
		require(faces_right(TO_THE_RIGHT, true) == false && faces_right(TO_THE_LEFT, true) == true,
				"the other way while confused");
		require(faces_right(CHARGE_LEFT, false) == false && faces_right(STEP_RIGHT, false) == true
				&& faces_right(STEP_RIGHT, true) == false, "Final Charge and Combat Step too");
		require(!faces_right(DOWN_TWICE, false) && !faces_right(FINAL_BLOW, false), "the rest don't turn");

		require(doing("swingP1PoleArm") == Doing::SWING && doing("swingP1") == Doing::OTHER
				&& doing("overSwingDouble") == Doing::DOUBLE && doing("fullSwingTriple") == Doing::TRIPLE
				&& doing("") == Doing::OTHER, "what the body is doing");
		require(std::string(polearm_swing(2112, POLEARM, 0)->action) == "swingT2PoleArm"
					&& polearm_swing(2000, POLEARM, 4)->stance == ms::Stance::Id::SWINGP1
					&& std::string(polearm_swing(2100, POLEARM, 2)->action) == "swingP2PoleArm"
					&& !polearm_swing(2112, 130, 0) && !polearm_swing(1100, POLEARM, 0),
				"an Aran's polearm swings");

		auto waiting = [](Outcome o) { return o.waits; };
		require(waiting(outcome(DOUBLE_SWING, true, Doing::SWING, std::nullopt))
				&& !waiting(outcome(DOUBLE_SWING, true, Doing::OTHER, std::nullopt))
				&& !waiting(outcome(DOUBLE_SWING, true, Doing::SWING, TO_THE_LEFT))
				&& !waiting(outcome(DOUBLE_SWING, false, Doing::SWING, std::nullopt)),
				"Double Swing on an attack with the polearm, before any other");
		require(waiting(outcome(TRIPLE_SWING, true, Doing::DOUBLE, std::nullopt))
				&& waiting(outcome(TRIPLE_SWING, true, Doing::OTHER, DOUBLE_SWING))
				&& !waiting(outcome(TRIPLE_SWING, true, Doing::DOUBLE, ROLLING_SPIN))
				&& !waiting(outcome(TRIPLE_SWING, true, Doing::SWING, std::nullopt)),
				"Triple Swing on or after a Double Swing");
		require(waiting(outcome(CHARGE_RIGHT, true, Doing::TRIPLE, std::nullopt))
				&& waiting(outcome(FINAL_TOSS, true, Doing::OTHER, TRIPLE_SWING))
				&& !waiting(outcome(FINAL_BLOW, true, Doing::DOUBLE, std::nullopt)),
				"the finals on or after a Triple Swing");
		require(!outcome(CHARGE_LEFT, true, Doing::TRIPLE, std::nullopt).takes
				&& outcome(FINAL_BLOW, true, Doing::OTHER, std::nullopt).takes
				&& !outcome(FINAL_BLOW, false, Doing::OTHER, std::nullopt).takes,
				"Final Blow takes the key even when it doesn't wait, if known");
		require(waiting(outcome(ROLLING_SPIN, true, Doing::OTHER, DOUBLE_SWING))
				&& outcome(STEP_LEFT, true, Doing::OTHER, std::nullopt).takes
				&& outcome(RAGE, true, Doing::OTHER, std::nullopt).takes
				&& !outcome(TO_THE_LEFT, true, Doing::OTHER, std::nullopt).takes,
				"the rest always wait");

		auto only = [](std::vector<int32_t> ids)
		{
			return [ids](int32_t id)
			{
				for (int32_t allowed : ids)
					if (allowed == id)
						return true;

				return false;
			};
		};
		auto combos = only({ TO_THE_LEFT, TO_THE_RIGHT, DOWN_TWICE });
		auto every = [](int32_t) { return true; };
		std::vector<int32_t> made;
		auto feed = [&](Commands& c, auto enabled, Key key, bool down, int64_t now)
		{
			made.clear();
			c.feed(static_cast<int64_t>(key), key, down, now, enabled, [&](int32_t id)
			{
				made.push_back(id);
				return false;
			});

			return made.empty() ? 0 : made.back();
		};
		auto press = [&](Commands& c, Key key, bool down, int64_t now)
		{
			return feed(c, combos, key, down, now);
		};

		Commands c;
		require(press(c, Key::DOWN, true, 0) == 0 && press(c, Key::LEFT, true, 100) == 0, "not yet");
		require(press(c, Key::ATTACK, true, 200) == TO_THE_LEFT, "Down, Left, attack");

		require(press(c, Key::DOWN, true, 1000) == 0 && press(c, Key::DOWN, false, 1050) == 0
				&& press(c, Key::LEFT, false, 1060) == 0 && press(c, Key::RIGHT, true, 1100) == 0
				&& press(c, Key::RIGHT, false, 1150) == 0, "keys let go between don't matter");
		require(press(c, Key::ATTACK, true, 1200) == TO_THE_RIGHT, "Down, Right, attack");

		require(press(c, Key::DOWN, true, 2000) == 0 && press(c, Key::RIGHT, true, 2100) == 0, "begun");
		require(press(c, Key::ATTACK, true, 2601) == 0, "but the whole of it must come within 600 ms");

		press(c, Key::ATTACK, false, 2700);
		require(press(c, Key::DOWN, true, 3000) == 0 && press(c, Key::OTHER, true, 3050) == 0
				&& press(c, Key::LEFT, true, 3100) == 0, "begun and broken");
		require(press(c, Key::ATTACK, true, 3200) == 0, "by any other key going down");

		press(c, Key::ATTACK, false, 3300);
		press(c, Key::LEFT, false, 3300);
		require(press(c, Key::DOWN, true, 4000) == 0 && press(c, Key::DOWN, false, 4100) == 0
				&& press(c, Key::DOWN, true, 4200) == 0, "Down, let go, Down");
		require(press(c, Key::ATTACK, true, 4300) == DOWN_TWICE, "and attack");

		press(c, Key::ATTACK, false, 4400);
		require(press(c, Key::DOWN, true, 5000) == 0 && press(c, Key::DOWN, true, 5050) == 0
				&& press(c, Key::DOWN, true, 5100) == 0, "a key held repeats its press, which doesn't count");
		require(press(c, Key::ATTACK, true, 5200) == 0, "so holding Down isn't down twice");

		press(c, Key::ATTACK, false, 5300);
		press(c, Key::DOWN, false, 5300);
		require(press(c, Key::DOWN, true, 6000) == 0 && press(c, Key::DOWN, false, 6100) == 0
				&& press(c, Key::DOWN, true, 6200) == 0 && press(c, Key::LEFT, true, 6300) == 0,
				"Down pressed again doesn't break the sideways ones");
		require(press(c, Key::ATTACK, true, 6400) == TO_THE_LEFT, "so they still match");

		Commands refused;
		auto smash_only = only({ TO_THE_LEFT, TO_THE_RIGHT });
		feed(refused, smash_only, Key::DOWN, true, 0);
		feed(refused, smash_only, Key::DOWN, false, 50);
		feed(refused, smash_only, Key::DOWN, true, 100);
		require(feed(refused, smash_only, Key::ATTACK, true, 150) == 0, "one not enabled never begins");

		Commands dead;
		press(dead, Key::DOWN, true, 0);
		press(dead, Key::LEFT, true, 50);
		dead.clear();
		require(press(dead, Key::ATTACK, true, 100) == 0, "cleared, it begins again");

		auto key = [&](Commands& k, Key which, bool down, int64_t now)
		{
			return feed(k, every, which, down, now);
		};

		Commands swings;
		require(key(swings, Key::ATTACK, true, 0) == 0 && key(swings, Key::ATTACK, false, 100) == 0
				&& key(swings, Key::ATTACK, true, 300) == DOUBLE_SWING, "the attack key twice");
		require(key(swings, Key::ATTACK, false, 400) == 0 && key(swings, Key::ATTACK, true, 899) == TRIPLE_SWING,
				"three times, within 900 ms of the first");

		Commands late;
		key(late, Key::ATTACK, true, 0);
		key(late, Key::ATTACK, false, 100);
		require(key(late, Key::ATTACK, true, 481) == 0, "the second within 480 ms");

		Commands later;
		key(later, Key::ATTACK, true, 0);
		key(later, Key::ATTACK, false, 100);
		key(later, Key::ATTACK, true, 300);
		key(later, Key::ATTACK, false, 400);
		require(key(later, Key::ATTACK, true, 901) == 0, "the third within 900 ms");

		Commands no_triple;
		auto doubles = only({ DOUBLE_SWING });
		feed(no_triple, doubles, Key::ATTACK, true, 0);
		feed(no_triple, doubles, Key::ATTACK, false, 100);
		feed(no_triple, doubles, Key::ATTACK, true, 300);
		feed(no_triple, doubles, Key::ATTACK, false, 400);
		require(feed(no_triple, doubles, Key::ATTACK, true, 500) == DOUBLE_SWING,
				"no Triple Swing unknown, but the second and third presses are another Double Swing");

		Commands charge;
		require(key(charge, Key::LEFT, true, 0) == 0 && key(charge, Key::LEFT, false, 50) == 0
				&& key(charge, Key::ATTACK, true, 600) == CHARGE_LEFT, "Left, then attack, within 600 ms");
		key(charge, Key::ATTACK, false, 700);
		key(charge, Key::RIGHT, true, 1000);
		require(key(charge, Key::ATTACK, true, 1601) == 0, "not later");

		Commands finals;
		require(key(finals, Key::DOWN, true, 0) == 0 && key(finals, Key::ATTACK, true, 300) == FINAL_BLOW,
				"Down, then attack");
		key(finals, Key::ATTACK, false, 350);
		key(finals, Key::DOWN, false, 350);
		require(key(finals, Key::UP, true, 1000) == 0 && key(finals, Key::ATTACK, true, 1100) == FINAL_TOSS,
				"Up, then attack");
		key(finals, Key::ATTACK, false, 1200);
		key(finals, Key::UP, false, 1200);
		require(key(finals, Key::UP, true, 2000) == 0 && key(finals, Key::ATTACK, true, 2301) == 0,
				"within 300 ms");

		Commands spin;
		require(key(spin, Key::UP, true, 0) == 0 && key(spin, Key::UP, false, 100) == 0
				&& key(spin, Key::UP, true, 200) == 0 && key(spin, Key::ATTACK, true, 650) == ROLLING_SPIN,
				"Up, Up, attack, within 700 ms");
		key(spin, Key::ATTACK, false, 700);
		key(spin, Key::UP, false, 700);
		key(spin, Key::UP, true, 1000);
		key(spin, Key::UP, false, 1100);
		key(spin, Key::UP, true, 1200);
		require(key(spin, Key::ATTACK, true, 1701) == 0, "not later");

		Commands steps;
		require(key(steps, Key::LEFT, true, 0) == 0 && key(steps, Key::LEFT, false, 100) == 0
				&& key(steps, Key::LEFT, true, 300) == STEP_LEFT, "a double tap within 300 ms");
		key(steps, Key::LEFT, false, 400);
		key(steps, Key::RIGHT, true, 1000);
		key(steps, Key::RIGHT, false, 1100);
		require(key(steps, Key::RIGHT, true, 1301) == 0, "not later");

		Commands rage;
		key(rage, Key::DOWN, true, 0);
		key(rage, Key::UP, true, 100);
		key(rage, Key::ATTACK, true, 200);
		require(std::find(made.begin(), made.end(), RAGE) != made.end(), "Down, Up, attack");

		Commands taking;
		taking.feed(1, Key::DOWN, true, 0, every, [](int32_t) { return false; });
		require(taking.feed(2, Key::ATTACK, true, 100, every, [](int32_t id) { return id == FINAL_BLOW; }),
				"a command takes the key");

		Queue q;
		require(!q.due(0) && q.empty() && !q.last(), "nothing waits");
		q.push(TO_THE_LEFT, 100);
		require(!q.empty() && q.due(1099) == TO_THE_LEFT, "a command waits");
		require(!q.due(1100) && q.empty(), "for a second at most");
		q.push(DOUBLE_SWING, 2000);
		q.push(TRIPLE_SWING, 2000);
		require(q.last() == TRIPLE_SWING, "the last waiting");
		q.pop();
		q.pop();
		require(q.empty(), "and goes once used");
		q.push(FINAL_BLOW, 3000);
		require(q.due(4999) == FINAL_BLOW && !q.due(5000), "Final Blow waits two seconds");

		{
			using namespace ms::combat_step;

			require(impulse(650, false).vx == 650.0 && impulse(1000, true).vx == -1000.0 && impulse(800, false).vy == 0.0,
					"Combat Step pushes the level's x forward and none up");
			require(may_step(true, false, true, 1000, 1000, false), "from a foothold once ready");
			require(!may_step(false, false, true, 1000, 0, false) && !may_step(true, true, true, 1000, 0, false),
					"not in the air or on a rope");
			require(!may_step(true, false, false, 1000, 0, false), "not where the map forbids it");
			require(!may_step(true, false, true, 999, 1000, false) && !may_step(true, false, true, 1000, 0, true),
					"not too soon, nor while pushed");
		}
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
