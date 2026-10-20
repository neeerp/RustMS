#pragma once

#include <algorithm>
#include <cstdint>
#include <string>

// Turning into a Morph.nx template, as a morph item (221xxxx) or Oak Barrel does (reports 225, 225b).
namespace ms::morph
{
	constexpr int32_t OAK_BARREL = 5101007;

	// The template's action for a stance, by the stance's name. A template without it stands.
	inline std::string action(const std::string &stance)
	{
		if (stance == "walk1" || stance == "walk2")
			return "walk";

		if (stance == "prone" || stance == "proneStab")
			return "prone";

		if (stance == "jump" || stance == "ladder" || stance == "rope" || stance == "sit" || stance == "fly")
			return stance;

		return "stand";
	}

	// An item's morph moves at its template's speed and jump, clamped as the player's own are, whatever
	// the equipment and buffs give (FUN_0094d3d9).
	inline int32_t speed(int32_t speed)
	{
		return std::clamp<int32_t>(speed, 80, 140);
	}

	inline int32_t jump(int32_t jump)
	{
		return std::clamp<int32_t>(jump, 80, 123);
	}

	// Whether right-clicking a morph item's icon may cancel it: not while morphed, for one with
	// noCancelMouse (FUN_00a096af).
	inline bool may_cancel(int32_t buff, bool morphed, bool no_cancel_mouse)
	{
		return !(buff < 0 && -buff / 10000 == 221 && morphed && no_cancel_mouse);
	}

	// Whether a mob touching the barrel sees through it: `roll` is a random number, and the skill's `prop`
	// the percent less one (0x94b6fc).
	inline bool spotted(int32_t prop, uint32_t roll)
	{
		return static_cast<uint32_t>(prop) >= roll % 100;
	}

	// Oak Barrel drops onto the player and replaces the body 840 ms after the buff, and when it breaks the
	// body fades back in over 240 ms (FUN_00936ca1, FUN_00930b27).
	class Barrel
	{
	public:
		static constexpr int16_t DROP = 840;
		static constexpr int16_t BREAK = 240;

		// Start dropping a barrel that lands as `morph`.
		void drop(int32_t into)
		{
			timer = DROP;
			morph = into;
		}

		void crack()
		{
			timer = BREAK;
			morph = 0;
		}

		void stop()
		{
			timer = 0;
		}

		// Advance by `step` ms: the morph the barrel lands as this step, or 0.
		int32_t update(int16_t step)
		{
			if (timer <= 0)
				return 0;

			timer -= step;

			return timer <= 0 ? morph : 0;
		}

		// Whether it drops or breaks.
		bool busy() const
		{
			return timer > 0;
		}

		bool dropping() const
		{
			return busy() && morph > 0;
		}

		// How opaque the body is while the barrel breaks.
		float alpha() const
		{
			if (!busy() || dropping())
				return 1.0f;

			return timer > BREAK / 2 ? 0.3f : 0.7f;
		}

	private:
		int16_t timer = 0;
		int32_t morph = 0;
	};
}
