#include "../Character/Snowball.h"

#include <cstring>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	struct Writer
	{
		std::vector<int8_t> bytes;

		template <typename T>
		Writer& put(T value)
		{
			int8_t raw[sizeof(T)];
			std::memcpy(raw, &value, sizeof(T));
			bytes.insert(bytes.end(), raw, raw + sizeof(T));
			return *this;
		}

		ms::InPacket packet() const
		{
			return ms::InPacket(bytes.data(), bytes.size());
		}
	};

	// 109060000's snowBall node.
	ms::snowball::Rules field()
	{
		ms::snowball::Rules rules;
		rules.dx = 3;
		rules.x0 = 400;
		rules.x = -440;
		rules.xmin = -60;
		rules.xmax = 900;
		rules.lanes[0] = { 155, "Map/Obj/event.img/snowyRock/snowball/0", "Map/Obj/event.img/snowyRock/snowball/1" };
		rules.lanes[1] = { -84, "Map/Obj/event.img/snowyRock/snowball/0", "Map/Obj/event.img/snowyRock/snowball/1" };

		return rules;
	}

	// Rolls `ball` for `ms`, a v83 tick at a time, and gives the steps moved.
	std::vector<int32_t> roll(ms::snowball::Ball& ball, int32_t ms, const ms::snowball::Rules& rules)
	{
		std::vector<int32_t> moves;

		for (int32_t t = 0; t < ms; t += ms::snowball::TICK)
			if (int32_t moved = ball.roll(ms::snowball::TICK, rules))
				moves.push_back(moved);

		return moves;
	}
}

int main()
{
	try
	{
		using namespace ms;

		snowball::Rules rules = field();

		// The boxes: a ball's 160 wide about its image at (x, y - 81), a
		// snowman's 60 wide standing on its lane.
		snowball::Rect ball = rules.ball(0, 0);
		require(ball.left == 320 && ball.top == -6 && ball.right == 480 && ball.bottom == 159, "ball 0 at step 0");
		require(rules.ball(1, 10).left == 350 && rules.ball(1, 10).top == -245, "ball 1 ten steps on");
		snowball::Rect snowman = rules.snowman(1);
		require(snowman.left == -470 && snowman.top == -184 && snowman.right == -410 && snowman.bottom == -84,
			"snowman 1");
		require(snowman.contains(-470, -184) && !snowman.contains(-410, -100) && !snowman.contains(-440, -84),
			"left and top are in, right and bottom out");
		require(rules.ball_x(-60) == 220 && rules.ball_x(900) == 3100, "the ends of the lane");
		require(snowball::nx_path("Map/Obj/event.img/snowyRock/snowball/0") == "Obj/event.img/snowyRock/snowball/0",
			"paths are in Map.nx");

		// SNOWBALL_STATE: the first on the field has the damages.
		Writer first;
		first.put<uint8_t>(0).put<int32_t>(100).put<int32_t>(80);
		first.put<int16_t>(0).put<uint8_t>(8).put<int16_t>(5).put<uint8_t>(3);
		first.put<uint16_t>(10).put<uint16_t>(15).put<uint16_t>(45);
		InPacket firstrecv = first.packet();
		snowball::StatePacket entry = snowball::read_state(firstrecv, true);
		require(!firstrecv.available(), "the whole of the first SNOWBALL_STATE is read");
		require(entry.state == 0 && entry.hp[0] == 100 && entry.hp[1] == 80, "the state and the snowmen's HP");
		require(entry.steps[1] == 5 && entry.stages[0] == 8 && entry.stages[1] == 3, "each ball's step and stage");
		require(entry.damages && (*entry.damages)[0] == 10 && (*entry.damages)[2] == 45, "the damages");

		Writer later;
		later.put<uint8_t>(1).put<int32_t>(0).put<int32_t>(100);
		later.put<int16_t>(-3).put<uint8_t>(0).put<int16_t>(0).put<uint8_t>(0);
		InPacket laterrecv = later.packet();
		snowball::StatePacket running = snowball::read_state(laterrecv, false);
		require(!laterrecv.available() && !running.damages, "later ones have none");
		require(running.steps[0] == -3, "a step may be before 0");

		// HIT_SNOWBALL, and its wait.
		Writer hit;
		hit.put<uint8_t>(2).put<uint16_t>(45).put<uint16_t>(40000);
		InPacket hitrecv = hit.packet();
		snowball::HitPacket strong = snowball::read_hit(hitrecv);
		require(!hitrecv.available() && strong.what == 2 && strong.damage == 45 && strong.delay == 40000,
			"its target, damage and unsigned delay");

		snowball::Hits hits;
		hits.add(strong, 1000);
		hits.add({ 0, 10, 0 }, 1000);
		hits.add({ 4, 10, 0 }, 1000);
		require(hits.size() == 2, "a hit on no target is dropped");
		std::vector<snowball::Hit> now = hits.take_due(1000);
		require(now.size() == 1 && now[0].what == 0 && now[0].damage == 10, "one without a delay at once");
		require(hits.take_due(40999).empty() && hits.take_due(41000).size() == 1 && hits.size() == 0,
			"the other when its delay is up");

		// Set from a STATE: snapped, or caught up with.
		require(snowball::snaps(true, 1) && snowball::snaps(false, 0) && snowball::snaps(false, 2) &&
					!snowball::snaps(false, 1),
			"only a running game's later states are caught up with");
		snowball::Ball rolling;
		require(rolling.set_position(10, 2, true, rules) == 10 && rolling.step == 10 && rolling.stage == 2,
			"a snap moves the ball there");
		require(rolling.set_position(12, 0, false, rules) == 0 && rolling.step == 10 && rolling.pending == 6 &&
					rolling.stage == 0,
			"a catch-up leaves it, three behind for each step");
		require(rolling.set_position(901, 5, true, rules) == 0 && rolling.step == 10 && rolling.stage == 0,
			"a step past the lane is ignored");
		require(rolling.set_position(-61, 5, false, rules) == 0 && rolling.pending == 6, "and one before it");

		// Rolling at a pace: 150 ms a step, 2/3 of that while behind and 4/3
		// while ahead.
		require(snowball::step_time(0) == 150 && snowball::step_time(7) == 500 && snowball::step_time(8) == 0 &&
					snowball::step_time(9) == -500 && snowball::step_time(10) == 0 && snowball::step_time(-1) == 0,
			"the stages' times");
		snowball::Ball even;
		even.set_position(0, 0, true, rules);
		require(roll(even, 300, rules) == std::vector<int32_t>{ 1, 1 } && even.step == 2, "a step per 150 ms");
		snowball::Ball behind;
		behind.set_position(0, 0, true, rules);
		behind.set_position(1, 0, false, rules);
		// 100 ms a step while 3 behind, each step takes one off.
		require(roll(behind, 90, rules).empty() && roll(behind, 30, rules).size() == 1 && behind.pending == 2,
			"faster while the server's ball is ahead");
		snowball::Ball ahead;
		ahead.set_position(5, 0, true, rules);
		ahead.set_position(4, 0, false, rules);
		require(ahead.pending == -3, "three ahead");
		require(roll(ahead, 180, rules).empty() && roll(ahead, 30, rules).size() == 1 && ahead.pending == 0 &&
					ahead.step == 6,
			"slower while ahead, and the backlog never goes below 0");

		// At stage 8 it only catches up, a step per 500 ms.
		snowball::Ball waiting;
		waiting.set_position(3, 8, true, rules);
		require(roll(waiting, 3000, rules).empty() && waiting.time == 0, "level with the server it waits");
		waiting.set_position(5, 8, false, rules);
		require(roll(waiting, 480, rules).empty() && roll(waiting, 30, rules) == std::vector<int32_t>{ 1 } &&
					waiting.pending == 3,
			"behind it rolls on");
		require(roll(waiting, 510, rules) == std::vector<int32_t>{ 1 } && waiting.step == 5 && waiting.pending == 0,
			"until it is there");
		waiting.set_position(4, 8, false, rules);
		require(roll(waiting, 510, rules) == std::vector<int32_t>{ -1 } && waiting.step == 4,
			"and back a step if it is ahead");

		// Stage 9 rolls back.
		snowball::Ball back;
		back.set_position(0, 9, true, rules);
		require(roll(back, 510, rules) == std::vector<int32_t>{ -1 } && back.step == -1, "a step back per 500 ms");

		// It never leaves the lane.
		snowball::Ball end;
		end.set_position(900, 0, true, rules);
		end.time = 140;
		require(roll(end, 600, rules).empty() && end.step == 900 && end.time == 0, "at xMax it stays");

		// A swing hits the first box it meets: the balls, then the snowmen.
		coconut::Box swing{ 300, 100, 340, 130 };
		require(snowball::first_hit(rules, { 0, 0 }, swing) == 0, "ball 0");
		require(snowball::first_hit(rules, { 100, 0 }, swing) == -1, "a ball rolled on is out of reach");
		require(snowball::first_hit(rules, { 0, 0 }, { -480, -150, -460, -120 }) == 3, "snowman 1");
		require(snowball::first_hit(rules, { 0, 0 }, { -470, -84, -410, -60 }) == -1, "touching edges don't meet");

		std::array<uint16_t, 3> damages = { 10, 15, 45 };
		require(snowball::damage(snowball::BALL_1, damages, 0) == 10, "a ball always takes its damage");
		require(snowball::damage(snowball::SNOWMAN_0, damages, 19) == 0 &&
					snowball::damage(snowball::SNOWMAN_0, damages, 20) == 15 &&
					snowball::damage(snowball::SNOWMAN_1, damages, 89) == 15 &&
					snowball::damage(snowball::SNOWMAN_1, damages, 90) == 45,
			"a snowman's: a miss, a hit or a strong one");
		require(snowball::touch_damage(0) == 30 && snowball::touch_damage(10) == 40 && snowball::touch_damage(11) == 30,
			"a snowman's touch hurts 30 to 40");

		// SNOWBALL_MESSAGE's lines.
		require(*snowball::message(0, 2) == "story Team's snowball has passed the 2 stage.", "a section passed");
		require(*snowball::message(1, 4) ==
					"story Team is attacking the snowman, stopping the progress of maple Team's snowball.",
			"the attackers, then the stopped");
		require(*snowball::message(1, 5) == "maple Team's snowball is moving again.", "moving again");
		require(!snowball::message(0, 0) && !snowball::message(0, 6), "no others");

		require(snowball::hp_width(100) == 46 && snowball::hp_width(50) == 23 && snowball::hp_width(1) == 0 &&
					snowball::hp_width(0) == 46,
			"the HP bar, full at 0");
		require(snowball::fade_opacity(0) == 1.0f && snowball::fade_opacity(50) == 0.5f &&
					snowball::fade_opacity(200) == 0.0f,
			"a fade's 100 ms");

		// The field's state: the first STATE snaps the balls and sets the
		// damages; a running game's catch up; the end fades a ball.
		snowball::enter_field(0, rules);
		require(!snowball::on_field(), "not a snowball field");
		snowball::enter_field(snowball::FIELD_TYPE, rules);
		require(snowball::on_field() && snowball::state().state == -1 && snowball::rules().x0 == 400, "on the field");
		entry.state = 1;
		snowball::apply(entry, 500);
		snowball::State& current = snowball::state();
		require(current.state == 1 && current.balls[1].step == 5 && current.balls[1].pending == 0 &&
					current.damages[1] == 15 && current.hp[1] == 80,
			"the first, though running, snaps");
		running.steps = { 2, 7 };
		snowball::apply(running, 600);
		require(current.balls[0].step == 0 && current.balls[0].pending == 6 && current.balls[1].pending == 6 &&
					current.damages[0] == 10 && current.hp[0] == 0,
			"then they catch up, and the damages stay");
		running.state = 3;
		snowball::apply(running, 700);
		require(current.balls[0].step == 2 && current.fades[1] == 700 && current.fades[0] == 0,
			"the end snaps them and fades the loser");
		current.hits.add({ 0, 10, 0 }, 0);
		snowball::enter_field(snowball::FIELD_TYPE, rules);
		require(snowball::state().state == -1 && snowball::state().hits.size() == 0 && snowball::state().fades[1] == 0,
			"a new field, a new game");
	}
	catch (const std::exception& e)
	{
		std::cerr << "snowball: " << e.what() << '\n';
		return 1;
	}

	std::cout << "snowball: ok\n";
	return 0;
}
