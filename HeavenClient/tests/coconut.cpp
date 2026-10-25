#include "../Character/Coconut.h"

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
}

int main()
{
	try
	{
		using namespace ms;

		require(coconut::object_name(0) == "000" && coconut::object_name(7) == "007", "names are three digits");
		require(coconut::object_name(500) == "500", "the last nut a swing can hit");

		// COCONUT_HIT: every nut, 5 s on, back to its first frame.
		Writer all;
		all.put<int16_t>(-1).put<uint16_t>(5000).put<uint8_t>(0);
		InPacket allrecv = all.packet();
		coconut::HitPacket start = coconut::read_hit(allrecv);
		require(!allrecv.available(), "the whole of COCONUT_HIT is read");
		require(start.index == -1 && start.delay == 5000 && start.state == 0, "its index, delay and state");

		// A delay past 32767 ms is still a delay.
		Writer late;
		late.put<int16_t>(12).put<uint16_t>(40000).put<uint8_t>(3);
		InPacket laterecv = late.packet();
		require(coconut::read_hit(laterecv).delay == 40000, "the delay is unsigned");

		// Every nut's state waits for its time, then comes in order.
		coconut::Hits hits;
		hits.add(start, 1000, 501);
		require(hits.size() == 501, "a negative index is each of the field's nuts");
		hits.add({ 7, 0, 1 }, 1000, 501);
		require(hits.take_due(5999).size() == 1, "a nut with no delay goes first");
		std::vector<coconut::Hit> shown = hits.take_due(6000);
		require(shown.size() == 501 && hits.size() == 0, "the rest as their delay runs out");
		require(shown.front().index == 0 && shown.back().index == 500 && shown[3].state == 0 && shown[3].due == 6000,
			"from nut 0 to nut 500");

		// A nut shaken and then brought down shows both, in that order.
		hits.add({ 4, 1000, 3 }, 0, 501);
		hits.add({ 4, 0, 1 }, 0, 501);
		hits.add({ 9, 500, 2 }, 0, 501);
		std::vector<coconut::Hit> first = hits.take_due(500);
		require(first.size() == 2 && first[0].index == 4 && first[0].state == 1 && first[1].index == 9,
			"what is due, in the order it came");
		std::vector<coconut::Hit> then = hits.take_due(1000);
		require(then.size() == 1 && then[0].state == 3, "then the fall");
		hits.add(start, 0, 3);
		hits.clear();
		require(hits.size() == 0 && hits.take_due(10000).empty(), "cleared");

		// COCONUT_SCORE: Maple's, then Story's.
		Writer score;
		score.put<uint16_t>(3).put<uint16_t>(40000);
		InPacket scorerecv = score.packet();
		auto scores = coconut::read_score(scorerecv);
		require(scores[0] == 3 && scores[1] == 40000 && !scorerecv.available(), "the two scores");

		// A swing hits the first nut whose box meets its own.
		std::vector<coconut::Box> nuts = {
			{ 0, 0, 0, 0 },			// no obj 000
			{ 100, 100, 130, 122 },
			{ 50, 100, 80, 122 },
			{ 60, 100, 90, 122 },
		};
		require(coconut::first_hit(nuts, { 40, 90, 100, 130 }) == 2, "the lowest index in reach");
		require(coconut::first_hit(nuts, { 85, 90, 100, 130 }) == 3, "only the boxes it meets");
		require(coconut::first_hit(nuts, { 80, 90, 100, 100 }) == -1, "touching edges don't meet");
		require(coconut::first_hit(nuts, { -10, -10, 10, 10 }) == -1, "an empty box is never hit");
		require(coconut::first_hit({}, { 0, 0, 100, 100 }) == -1, "no nuts");

		// The field's uniforms: team 0's, then team 1's, a man's before a woman's.
		std::array<coconut::Uniform, 4> uniforms = { { { 1002014, 1050040 }, { 1002014, 1051029 },
														{ 1002069, 1050041 }, { 1002069, 1051075 } } };
		coconut::enter_field(0, 501, uniforms);
		require(!coconut::on_field() && !coconut::uniform(0, false), "no uniform off the field");
		coconut::enter_field(coconut::FIELD_TYPE, 501, uniforms);
		require(coconut::on_field() && coconut::count() == 501, "on the field");
		require(coconut::uniform(0, false)->clothes == 1050040 && coconut::uniform(0, true)->clothes == 1051029,
			"Maple's");
		require(coconut::uniform(1, false)->cap == 1002069 && coconut::uniform(1, true)->clothes == 1051075,
			"Story's");
		require(!coconut::uniform(coconut::NO_TEAM, false), "no team, no uniform");

		// Entering a field forgets the last game.
		coconut::state().score = { 4, 2 };
		coconut::state().end = 9000;
		coconut::state().dressed = true;
		coconut::state().hits.add(start, 0, 501);
		coconut::enter_field(coconut::FIELD_TYPE, 501, uniforms);
		require(coconut::state().score[0] == 0 && coconut::state().end == 0 && !coconut::state().dressed &&
					coconut::state().hits.size() == 0,
			"a new field, a new game");

		// The scoreboard.
		require(coconut::score_text(0) == "000" && coconut::score_text(42) == "042" &&
					coconut::score_text(501) == "501",
			"scores are three digits");
		require(coconut::clock_end(1000, 300) == 301000, "the clock runs out after its seconds");
		require(coconut::clock_end(-5000, 5) == 1, "never at 0");
		require(!coconut::time_text(0, 5000), "no time before a clock");
		require(*coconut::time_text(301000, 1000) == "05:00", "the minutes and seconds left");
		require(*coconut::time_text(301000, 1001) == "04:59", "a second begun is not left");
		require(*coconut::time_text(301000, 240500) == "01:00", "60.5 s is a minute");
		require(*coconut::time_text(301000, 400000) == "00:00", "00:00 once it has run out");
	}
	catch (const std::exception& e)
	{
		std::cerr << "coconut: " << e.what() << '\n';
		return 1;
	}

	std::cout << "coconut: ok\n";
	return 0;
}
