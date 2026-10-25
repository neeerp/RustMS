#pragma once

#include "../Net/InPacket.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// Coconut Season (CField_Coconut, v83ref b167): two teams knock the
	// field's coconuts down. The server says what each nut does; the client
	// shows it, keeps the score and the clock on its scoreboard, dresses the
	// player in their team's uniform, and tells the server which nut a swing
	// hit.
	namespace coconut
	{
		constexpr int32_t FIELD_TYPE = 4;
		// FIELD_SPECIFIC_DATA's team for a player on neither.
		constexpr uint8_t NO_TEAM = 0xff;
		// The nuts of a field whose coconut block gives no counts.
		constexpr int16_t DEFAULT_COUNT = 501;

		// A coconut's map obj: "%03d" of its index (FUN_00549eb7). The objs
		// are named 001 to 501, so nut 0 has none and obj 501 is never used.
		std::string object_name(int16_t index);

		// A nut's state after `due` (in ms, by the clock COCONUT_HIT came by).
		struct Hit
		{
			int16_t index;
			uint8_t state;
			int64_t due;
		};

		// COCONUT_HIT (0x11D): a nut, or every one for a negative index.
		struct HitPacket
		{
			int16_t index = 0;
			uint16_t delay = 0;
			uint8_t state = 0;
		};

		HitPacket read_hit(InPacket& recv);

		// COCONUT_SCORE (0x11E): team 0's (Maple) and team 1's (Story).
		std::array<uint16_t, 2> read_score(InPacket& recv);

		// The states waiting for their time, kept in the order they came.
		class Hits
		{
		public:
			// A hit at `now` on one of `count` nuts, or on all of them.
			void add(const HitPacket& hit, int64_t now, int16_t count);
			// Those due by `now`, which are taken off.
			std::vector<Hit> take_due(int64_t now);
			size_t size() const;
			void clear();

		private:
			std::vector<Hit> pending;
		};

		// A box on the map, by its edges.
		struct Box
		{
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;

			bool empty() const;
		};

		// The first nut whose box meets the swing's (IntersectRect), or -1:
		// FUN_0095719b as the coconut field calls it, which skips empty boxes.
		int16_t first_hit(const std::vector<Box>& nuts, const Box& swing);

		// The uniform of a team for a gender: cap and overall.
		struct Uniform
		{
			int32_t cap = 0;
			int32_t clothes = 0;
		};

		// The field the server has moved the player to, as for the carnival:
		// the coconut packets come before the field loads. Entering a field
		// forgets the game.
		void enter_field(int32_t fieldtype, int16_t count, const std::array<Uniform, 4>& uniforms);
		// Whether that field is a coconut one.
		bool on_field();
		int16_t count();

		// The uniform of `team` for the gender, or none for no team
		// (slot (team != 0) * 2 + female, FUN_005497b4).
		std::optional<Uniform> uniform(uint8_t team, bool female);

		// What the scoreboard shows.
		struct State
		{
			std::array<uint16_t, 2> score = { 0, 0 };
			// When the clock runs out, in ms, or 0 before a CLOCK.
			int64_t end = 0;
			Hits hits;
			// Whether the player wears their team's uniform.
			bool dressed = false;
		};

		State& state();

		// The time hits and the clock go by, in milliseconds.
		int64_t now();

		// CLOCK on the field (FUN_005498c6): the game ends `seconds` from
		// `now`. An end of 0 would mean no clock, so it becomes 1.
		int64_t clock_end(int64_t now, int32_t seconds);

		// "%03d", as the scores are drawn.
		std::string score_text(uint16_t score);
		// "%02d:%02d" of the whole seconds left, 00:00 once the clock has
		// run out, or nothing before a CLOCK (FUN_00549a13).
		std::optional<std::string> time_text(int64_t end, int64_t now);
	}
}
