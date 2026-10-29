#pragma once

#include "DojoEnergy.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// Nett's Pyramid and the Dusty Platform: their stages (CField_Massacre, field type 23), with a gauge that
	// runs down, a panel counting kills, cool kills and misses, and a clock of their own, and the result room
	// (CField_MassacreResult, 24), which shows a run's rank and EXP.
	namespace massacre
	{
		constexpr int32_t FIELD_STAGE = 23;
		constexpr int32_t FIELD_RESULT = 24;

		// What the server last said. The counts (SESSION_VALUE `massacre_*`, the context's +0x38a4 to +0x38b0)
		// outlast the field, for the result room; the gauge's count (PYRAMID_GAUGE, +0x854) and the clock
		// (CLOCK kind 2, +0x850) go with it.
		struct Result
		{
			int8_t rank;
			int32_t exp;
		};

		struct Session
		{
			// The field type of the field being entered or on.
			int32_t field = 0;
			int32_t hit = 0;
			int32_t miss = 0;
			int32_t cool = 0;
			// Uses of Rage of Pharaoh left.
			int32_t skill = 0;
			int32_t count = 0;
			// The stage's length, in seconds.
			int32_t length = 0;
			dojo_energy::Countdown clock;
			// Whether the field has loaded, and a result that came before it did.
			bool loaded = false;
			std::optional<Result> result;
		};

		inline Session &session()
		{
			static Session state;

			return state;
		}

		// A new field: the gauge and the clock start over, the counts stay.
		inline void enter_field(Session &state, int32_t fieldtype)
		{
			state.field = fieldtype;
			state.count = 0;
			state.length = 0;
			state.clock = {};
			state.loaded = false;
			state.result.reset();
		}

		// PYRAMID_SCORE: the result to show now. One that comes while the result room is still loading waits
		// for it, as v83 reads the packet in the room, and shows for its whole time there.
		inline std::optional<Result> score(Session &state, Result result)
		{
			if (state.loaded)
				return result;

			state.result = result;

			return std::nullopt;
		}

		// The field has loaded: the result that waited for it, if any.
		inline std::optional<Result> field_loaded(Session &state)
		{
			state.loaded = true;

			std::optional<Result> result = state.result;
			state.result.reset();

			return result;
		}

		// The stage's clock (CLOCK kind 2, 0x560389).
		inline void start_clock(Session &state, int64_t now_ms, int32_t seconds)
		{
			state.length = seconds;
			state.clock.start(now_ms, seconds);
		}

		// Keeps a SESSION_VALUE that is one of the counts (FUN_00a12bd6); false for any other key.
		inline bool store(Session &state, const std::string &key, int32_t value)
		{
			if (key == "massacre_hit")
				state.hit = value;
			else if (key == "massacre_miss")
				state.miss = value;
			else if (key == "massacre_cool")
				state.cool = value;
			else if (key == "massacre_skill")
				state.skill = value;
			else
				return false;

			return true;
		}

		// Rage of Pharaoh goes only with a use left (0x74d841).
		inline bool rage_ready(int32_t uses)
		{
			return uses > 0;
		}

		// How far the gauge has run down (0x5602c0): `decrease` for each second gone, less the count the
		// server sent, kept within 0 and `total`.
		inline int32_t gauge(int32_t length, int32_t left, int32_t decrease, int32_t count, int32_t total)
		{
			int32_t value = (length - left) * decrease - count;

			if (value > total)
				value = total;

			return value < 0 ? 0 : value;
		}

		// The bar (0x560d07): up to 259 pixels, growing leftwards from x 540, with the icon at its head 10
		// pixels further left. It turns red from 65%.
		constexpr int16_t GAUGE_WIDTH = 259;
		constexpr int16_t GAUGE_RIGHT = 540;
		constexpr int16_t ICON_RIGHT = 530;

		inline int16_t gauge_width(int32_t value, int32_t total)
		{
			return total > 0 ? static_cast<int16_t>(value * GAUGE_WIDTH / total) : 0;
		}

		inline bool danger(int32_t value, int32_t total)
		{
			return total > 0 && value * 100 / total >= 65;
		}

		// The digits of `value` in a row of `slots` (0x5511ff): the last in the last slot and each before it
		// one slot back, with no leading zeros; 0 shows one 0. Each comes as its slot and digit.
		inline std::vector<std::pair<int32_t, int32_t>> digits(int32_t value, int32_t slots)
		{
			std::vector<std::pair<int32_t, int32_t>> shown;

			if (value < 0)
				value = 0;

			for (int32_t slot = slots - 1; slot >= 0; slot--)
			{
				shown.emplace_back(slot, value % 10);
				value /= 10;

				if (value == 0)
					break;
			}

			return shown;
		}

		// The Count panel (0x560b25), and where each row's number starts in it. The skill's row is there
		// only on maps that allow the skill (`mobMassacre/disableSkill` 0).
		constexpr int32_t COUNT_SLOTS = 5;
		constexpr int16_t COUNT_ADVANCE = 12;
		constexpr int16_t COUNT_X = 57;
		constexpr std::array<int16_t, 3> COUNT_Y = { 11, 33, 55 };
		constexpr int16_t SKILL_ADVANCE = 17;
		constexpr int16_t SKILL_X = 80;
		constexpr int16_t SKILL_Y = 85;

		// The key under the panel (0x560a97, 0x560e85): it opens when the first use comes, stays while there
		// is one, and closes when the last is gone.
		class KeyPanel
		{
		public:
			enum class State
			{
				HIDDEN,
				OPENING,
				SHOWN,
				CLOSING
			};

			void skill_changed(int32_t from, int32_t to)
			{
				if (from == 0 && to > 0)
					state = State::OPENING;
				else if (from > 0 && to == 0)
					state = State::CLOSING;
			}

			// The opening or closing played through.
			void finished()
			{
				if (state == State::OPENING)
					state = State::SHOWN;
				else if (state == State::CLOSING)
					state = State::HIDDEN;
			}

			State get() const
			{
				return state;
			}

		private:
			State state = State::HIDDEN;
		};

		// The stage's clock (0x5605f2): the minutes and the seconds left, each as `%02d`.
		inline std::array<std::string, 2> clock_text(int32_t seconds)
		{
			if (seconds < 0)
				seconds = 0;

			auto two = [](int32_t value) { return (value < 10 ? "0" : "") + std::to_string(value); };

			return { two(seconds / 60), two(seconds % 60) };
		}

		// The result room's share of each count (0x5617c5), of them all or of 1 with none.
		inline int32_t percent(int32_t value, int32_t hit, int32_t miss, int32_t cool)
		{
			int32_t total = hit + miss + cool;

			return value * 100 / (total > 1 ? total : 1);
		}
	}
}
