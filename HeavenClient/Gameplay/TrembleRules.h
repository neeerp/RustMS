#pragma once

#include <algorithm>
#include <cstdint>

// How v83 shakes the field's view: FIELD_EFFECT 1, some bosses appearing or dying, mob attacks with a
// `tremble` flag and Dragon Roar (CAnimationDisplayer::SetTremble 0x43da05, updated by FUN_004375b1).
namespace ms::tremble
{
	// v83 moves the view once per this many ms.
	constexpr int32_t TICK = 30;

	struct State
	{
		double force = 0;
		int32_t start = 0;
		int32_t end = 0;
		double factor = 0;
	};

	struct Offset
	{
		int32_t x;
		int32_t y;

		bool operator==(const Offset &other) const
		{
			return x == other.x && y == other.y;
		}
	};

	// A new shake replaces the last one. Without an `additional` time it fades, a heavy one faster.
	inline State set(double force, bool heavy, int32_t delay, int32_t additional, int32_t now)
	{
		State state;
		state.force = force;
		state.start = now + delay;
		state.end = now + delay + additional + (heavy ? 1500 : 2000);
		state.factor = additional ? 1.0 : heavy ? 0.85 : 0.92;

		return state;
	}

	// One tick at `now`, with two rand()s: the view's offset from where it should be, which replaces the
	// last tick's.
	inline Offset step(State &state, int32_t now, uint32_t rx, uint32_t ry)
	{
		if (state.force <= 0 || now <= state.start)
			return { 0, 0 };

		auto span = static_cast<uint32_t>(static_cast<int32_t>(2 * state.force));
		auto low = static_cast<int32_t>(-state.force);
		Offset offset{ static_cast<int32_t>(rx % span) + low, static_cast<int32_t>(ry % span) + low };

		state.force *= state.factor;

		if (state.force < 1)
			state.force = 0;

		if (now >= state.end || state.force == 0)
		{
			state.force = 0;

			return { 0, 0 };
		}

		return offset;
	}

	// The skills that shake the view as they're used, the user's own (0x951a7c) or another's (0x980808).
	inline bool skill_shakes(int32_t skill, bool own)
	{
		switch (skill)
		{
			case 1111008: // Shout
			case 1311006: // Dragon Roar
			case 9001001: // Super Dragon Roar
			case 1009: // Bamboo Rain
			case 10001009:
			case 20001009:
			case 20011009:
			case 1020: // Rage of Pharaoh
			case 10001020:
			case 20001020:
				return true;
			// v83 leaves it out of the user's own list.
			case 20011020:
				return !own;
			default:
				return false;
		}
	}

	// Shout shakes heavy and harder than the rest.
	inline bool skill_heavy(int32_t skill)
	{
		return skill == 1111008;
	}

	// Less the further the user is, to nothing at 400 px.
	inline double skill_force(int32_t skill, double distance)
	{
		return (400 - std::min(distance, 400.0)) / (skill_heavy(skill) ? 15 : 25);
	}
}
