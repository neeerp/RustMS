#pragma once

#include <algorithm>
#include <cstdint>
#include <string>

// Turning into a Morph.nx template, as a morph item (221xxxx) does (report 225).
namespace ms::morph
{
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
}
