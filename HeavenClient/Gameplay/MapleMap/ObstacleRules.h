#pragma once

#include <cmath>
#include <cstdint>

// How v83 hurts a player who touches a map obj marked `obstacle`: spikes, lasers, steam and the like
// (FUN_00642264 builds their boxes each tick, FUN_00642710 tests them).
namespace ms::obstacle
{
	// What an obstacle obj's node says (FUN_0063c741).
	struct Trap
	{
		int32_t damage = 0;
		int32_t mobdamage = 0;
		int32_t dir = 0;
		int32_t disease = 0;
		int32_t level = 0;
		// Hits leave the player on this much HP, when it's 0 or more.
		int32_t force_hp = -1;
		// The map a touch sends the player to, when not 0.
		int32_t target_field = 0;
	};

	struct Vec
	{
		int32_t x;
		int32_t y;

		bool operator==(const Vec &other) const
		{
			return x == other.x && y == other.y;
		}
	};

	struct Box
	{
		int32_t left;
		int32_t top;
		int32_t right;
		int32_t bottom;

		bool operator==(const Box &other) const
		{
			return left == other.left && top == other.top && right == other.right && bottom == other.bottom;
		}

		// IntersectRect: the boxes share some area.
		bool meets(const Box &other) const
		{
			return left < other.right && other.left < right && top < other.bottom && other.top < bottom;
		}
	};

	// The push of a trap whose `dir` is a clock direction (0x642500). v83 passes 90 - 30 * dir to cos and sin
	// as radians, so 3 pushes right but 6, 9 and 12 go wherever that lands.
	inline Vec push(int32_t dir, bool flip)
	{
		if (dir == 0)
			return { 0, 0 };

		double angle = 90.0 - 30.0 * dir;
		auto x = static_cast<int32_t>(400.0 * std::cos(angle));
		auto y = static_cast<int32_t>(-400.0 * std::sin(angle));

		return { flip ? -x : x, y };
	}

	// The box of a frame with `lt` and `rb` on an obj at (x, y), or false when the frame has none or it's
	// under 5 px both ways (0x642491). A flipped obj mirrors it.
	inline bool box(int32_t x, int32_t y, Vec lt, Vec rb, bool flip, Box &out)
	{
		if (rb.x - lt.x < 5 && rb.y - lt.y < 5)
			return false;

		out = flip ? Box{ x - rb.x, y + lt.y, x - lt.x, y + rb.y } : Box{ x + lt.x, y + lt.y, x + rb.x, y + rb.y };

		return true;
	}

	// A hit's damage from the trap's, with `random` a rand() (0x6427c8): within a tenth either way.
	inline int32_t roll(int32_t damage, uint32_t random)
	{
		if (damage <= 10)
			return damage;

		return static_cast<int32_t>(random % static_cast<uint32_t>(2 * (damage / 10) + 1)) + damage - damage / 10;
	}

	// A `forceHP` trap takes the player down to it, or hits for all their HP when they're at or below it.
	inline int32_t forced(int32_t damage, int32_t hp, int32_t force_hp)
	{
		if (force_hp < 0)
			return damage;

		return hp > force_hp ? hp - force_hp : hp;
	}

	// The disease and its level that a hit reports, as the server's mob skill code (0x6427fe).
	inline uint16_t code(int32_t damage, const Trap &trap)
	{
		if (damage < 1 || trap.disease == 0 || trap.level == 0)
			return 0;

		return static_cast<uint16_t>(trap.disease << 8 | trap.level);
	}

	// The push of a trap without one (0x94b610): back against the player's step and up unless they're
	// rising. v83 takes the position before the step from the one after it (I: vt+0x10 as the earlier).
	inline Vec fallback(int32_t last_x, int32_t last_y, int32_t x, int32_t y)
	{
		int32_t dx = last_x - x;
		int32_t dy = last_y - y;

		return { dx < 0 ? -270 : dx > 0 ? 270 : 0, dy > 0 ? 0 : -270 };
	}
}
