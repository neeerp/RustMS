#pragma once

#include <cstdint>

namespace ms::beacon
{
	constexpr int32_t HOMING_BEACON = 5211006;
	constexpr int32_t BULLSEYE = 5220011;

	// The mob a Homing Beacon marked and the skill that did, until the
	// server takes the mark off (GuidedBullet, bit 87).
	struct Mark
	{
		int32_t skill = 0;
		int32_t mob = 0;
	};

	// A Corsair who learned Bullseye fires it in Homing Beacon's place
	// (0x9670b3).
	inline int32_t cast(int32_t skill, int32_t bullseye_level)
	{
		return skill == HOMING_BEACON && bullseye_level > 0 ? BULLSEYE : skill;
	}

	// Whether a shot or spell looks for the marked mob before the first in
	// its path (FUN_009537d5): all but the beacons, which mark the first.
	inline bool guides(int32_t skill, const Mark &mark)
	{
		return mark.mob != 0 && skill != HOMING_BEACON && skill != BULLSEYE;
	}

	// A hit on mob `oid`, with Bullseye's `x` percent more if it is the
	// marked one (FUN_0078dea6).
	inline double bullseye(double damage, int32_t x, int32_t marked, int32_t oid)
	{
		return x > 0 && marked != 0 && oid == marked ? (x + 100) * damage * 0.01 : damage;
	}
}
