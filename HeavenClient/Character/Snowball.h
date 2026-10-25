#pragma once

#include "Coconut.h"

#include "../Net/InPacket.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// Snowball (CField_SnowBall, v83ref b168): two teams push their giant
	// snowball along their own lane, and bring down the snowman in it to
	// stop the other team's. The server says where each ball is and what
	// each hit shows; the client rolls the balls on at the pace it is sent,
	// shows the hits, throws a player back from a ball, hurts one who walks
	// into a snowman, and tells the server what a swing hit.
	namespace snowball
	{
		constexpr int32_t FIELD_TYPE = 1;
		// v83's update comes every 30 ms, and each adds that to a ball's
		// time (A 57419a).
		constexpr int32_t TICK = 30;
		// A ball that only catches up takes a step per 500 ms (DAT_00bd8bc0).
		constexpr int32_t CATCH_UP_TIME = 500;
		// How long a ball takes to fade out: its canvas's default delay.
		constexpr int32_t FADE_TIME = 100;
		// A ball's turn per step, in degrees (A 573ec9).
		constexpr int32_t TURN_PER_STEP = 4;

		// What a hit is on: the two balls, then the two snowmen.
		enum Target : uint8_t
		{
			BALL_0,
			BALL_1,
			SNOWMAN_0,
			SNOWMAN_1,
			TARGETS
		};

		// A box by its edges; a point is in it as for PtInRect, left and top
		// in and right and bottom out.
		struct Rect
		{
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;

			bool contains(int32_t x, int32_t y) const;
		};

		// The field's snowBall node, as the client reads it (A 573db5,
		// A 574294).
		struct Lane
		{
			int32_t y = 0;
			// The canvases, as paths in Map.nx.
			std::string ball;
			std::string snowman;
		};

		struct Rules
		{
			// A step's width in pixels.
			int32_t dx = 3;
			// The ball's x at step 0, and the snowmen's.
			int32_t x0 = 0;
			int32_t x = 0;
			// The steps a ball may be at.
			int32_t xmin = 0;
			int32_t xmax = 0;
			std::array<Lane, 2> lanes;

			// A ball's box at a step: 160 wide, centred on its image at
			// (x, y - 81).
			Rect ball(size_t lane, int32_t step) const;
			// A snowman's: 60 wide and 100 high, standing on its lane.
			Rect snowman(size_t lane) const;
			// Where a ball's image is drawn.
			int32_t ball_x(int32_t step) const;
		};

		// A map.nx path of the map's ("Map/Obj/...") as one within the file.
		std::string nx_path(const std::string& path);

		// SNOWBALL_STATE (0x119).
		struct StatePacket
		{
			uint8_t state = 0;
			// The snowmen's HP, in percent.
			std::array<int32_t, 2> hp = { 0, 0 };
			std::array<int16_t, 2> steps = { 0, 0 };
			std::array<uint8_t, 2> stages = { 0, 0 };
			// In the first on the field only: what a hit on a ball shows, on
			// a snowman, and a strong one on a snowman.
			std::optional<std::array<uint16_t, 3>> damages;
		};

		StatePacket read_state(InPacket& recv, bool first);

		// HIT_SNOWBALL (0x11A).
		struct HitPacket
		{
			uint8_t what = 0;
			uint16_t damage = 0;
			uint16_t delay = 0;
		};

		HitPacket read_hit(InPacket& recv);

		// A hit to show at `due`, in ms.
		struct Hit
		{
			uint8_t what;
			uint16_t damage;
			int64_t due;
		};

		// The hits waiting for their time.
		class Hits
		{
		public:
			// A hit come at `now`; one on no target is dropped.
			void add(const HitPacket& hit, int64_t now);
			// Those due by `now`, in the order they came, which are taken off.
			std::vector<Hit> take_due(int64_t now);
			size_t size() const;

		private:
			std::vector<Hit> pending;
		};

		// A ball's roll (the 0x38-byte ball in CField_SnowBall). Each
		// returns how many steps the ball moved, which turns it as well.
		struct Ball
		{
			// The step it is at.
			int32_t step = 0;
			// How far the server's step is ahead of it, three per step.
			int32_t pending = 0;
			// Its pace (an index of the step times) and the time it has
			// gone without a step.
			int32_t stage = 0;
			int32_t time = 0;

			// From SNOWBALL_STATE (A 57414a): at once, or caught up with
			// over the next steps. A step out of the field's is ignored.
			int32_t set_position(int16_t to, uint8_t pace, bool snap, const Rules& rules);
			// `ms` of rolling at its pace, while the game runs (A 57419a).
			int32_t roll(int32_t ms, const Rules& rules);
		};

		// The ms per step of a stage: forward, 0 to only catch up, or back
		// for a negative one (DAT_00bd8ba4). A stage past the table only
		// catches up.
		int32_t step_time(int32_t stage);

		// Whether a STATE snaps the balls to their steps: all but one that
		// keeps a running game running.
		bool snaps(bool first, uint8_t state);

		// The first of the balls and snowmen whose box meets the swing's, as
		// for Coconut Season (A 575387).
		int16_t first_hit(const Rules& rules, const std::array<int32_t, 2>& steps, const coconut::Box& swing);

		// What a swing on `target` shows, by a roll in 0..99: the ball's
		// damage, or on a snowman 0 two times in ten, the strong one once
		// and the normal one otherwise.
		uint16_t damage(uint8_t target, const std::array<uint16_t, 3>& damages, int32_t roll);

		// A snowman's touch: 30 to 40 damage (A 574df1).
		int32_t touch_damage(int32_t roll);

		// SNOWBALL_MESSAGE (0x11B): the yellow line about a team's ball, or
		// nothing.
		std::optional<std::string> message(uint8_t team, uint8_t line);

		// The red of a snowman's HP bar, of 46 (A 574606); at 0 HP or less it
		// is full, as in v83.
		int32_t hp_width(int32_t hp);

		// A ball's opacity, of 1, `elapsed` ms into its fade.
		float fade_opacity(int64_t elapsed);

		struct State
		{
			// -1 before the first SNOWBALL_STATE.
			int32_t state = -1;
			std::array<int32_t, 2> hp = { 0, 0 };
			std::array<uint16_t, 3> damages = { 0, 0, 0 };
			std::array<Ball, 2> balls;
			// When each ball began to fade, or 0.
			std::array<int64_t, 2> fades = { 0, 0 };
			Hits hits;
		};

		// The field the server has moved the player to: the snowball
		// packets come before it loads. Entering a field forgets the game.
		void enter_field(int32_t fieldtype, const Rules& rules);
		bool on_field();
		const Rules& rules();
		State& state();

		// A SNOWBALL_STATE at `now`.
		void apply(const StatePacket& packet, int64_t now);

		// The time hits and fades go by, in milliseconds.
		int64_t now();
	}
}
