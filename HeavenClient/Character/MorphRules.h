#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <string>
#include <string_view>

// Turning into a Morph.nx template, as a morph item (221xxxx), Oak Barrel or Transformation does
// (reports 225, 225b, 225c).
namespace ms::morph
{
	constexpr int32_t OAK_BARREL = 5101007;
	constexpr int32_t SUPER_TRANSFORMATION = 5121003;
	constexpr int32_t ALBATROSS = 13111005;

	// The skills a superman can't use: Double Shot, Oak Barrel, Burst Fire, and the beginners' rides and
	// the like (0x9673cf).
	inline bool superman_refuses(int32_t skill)
	{
		if (skill == 5001003 || skill == OAK_BARREL || skill == 5210000)
			return true;

		int32_t id = skill % 10000;
		bool ride = id == 1004 || id == 1009 || (id >= 1027 && id <= 1031);

		switch (skill / 10000)
		{
			case 0:
				return ride || id == 1013 || (id >= 1017 && id <= 1020);
			case 1000:
				return ride || id == 1014 || id == 1019 || id == 1020 || id == 1022 || id == 1023;
			case 2000:
				return ride || id == 1019 || id == 1020 || id == 1022 || id == 1023;
			case 2001:
				return ride;
			default:
				return false;
		}
	}

	// Whether a skill wants a transformation the player lacks: Shockwave any, Demolition and Snatch
	// Super Transformation's, by the skill `reason` the player is morphed for (FUN_00969465).
	inline bool untransformed(int32_t skill, bool superman, int32_t reason)
	{
		switch (skill)
		{
			case 5111006:
			case 15111003:
				return !superman;
			case 5121004:
			case 5121005:
				return !superman || reason != SUPER_TRANSFORMATION;
			default:
				return false;
		}
	}

	// A superman's action for a character's attack or skill action, by name, or empty to draw the
	// stance's (FUN_00451b6a). v83 reads an unlisted action's index as a morph action; this doesn't.
	inline std::string superman_action(const std::string &action)
	{
		static const std::array<std::string_view, 32> SAME = {
			"swingT1", "swingT3", "stabO1", "stabO2", "shoot1", "shootF", "proneStab", "alert2",
			"alert3", "alert4", "ladder2", "rope2", "somersault", "straight", "eburster", "backspin",
			"eorb", "screw", "doubleupper", "dragonstrike", "doublefire", "edrain", "recovery", "fist",
			"timeleap", "wave", "stormbreak", "shockwave", "demolition", "snatch", "windspear", "windshot"
		};

		if (action == "alert5")
			return "alert";

		return std::find(SAME.begin(), SAME.end(), action) != SAME.end() ? action : "";
	}

	// The template's action for a stance, by the stance's name. A template without it stands. Only a
	// superman shows its alert and sits (FUN_00451ec8).
	inline std::string action(const std::string &stance, bool superman)
	{
		if (superman)
		{
			std::string attack = superman_action(stance);

			if (!attack.empty())
				return attack;

			if (stance == "alert" || stance == "sit")
				return stance;
		}

		if (stance == "walk1" || stance == "walk2")
			return "walk";

		if (stance == "prone" || stance == "proneStab")
			return "prone";

		if (stance == "jump" || stance == "ladder" || stance == "rope" || stance == "fly")
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
