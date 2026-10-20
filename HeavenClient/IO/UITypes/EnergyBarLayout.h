#pragma once

#include <cstdint>

// The Energy Charge gauge (v83 0x7fa3c9), from UIClassic's UIWindow.img/EnergyBar: an 82x10 frame of `w`, `c`
// columns and `e`, filled from x 2 with one column for each 78th of the energy.
namespace ms::energy_bar
{
	constexpr int32_t FULL = 10000;
	constexpr int16_t WIDTH = 82, HEIGHT = 10;
	// The frame's ends, `w` and `e`, are 6 wide.
	constexpr int16_t END = 6;
	// v83 draws the frame at alpha 0xb4.
	constexpr float FRAME_OPACITY = 180.0f / 255.0f;
	// The fill's first column; its caps, `Gage/1/0`, are at y 3, the columns between, `Gage/1/1`, at y 2.
	constexpr int16_t START = 2, CAP_Y = 3, COLUMN_Y = 2;

	// How many columns `energy` fills: ftol(energy / FULL * 78.0), none when it is 0.
	inline int32_t fill(int32_t energy)
	{
		if (energy <= 0)
			return 0;

		return static_cast<int32_t>(static_cast<double>(energy) / FULL * 78.0);
	}

	// Whether the fill's column at x is a cap: the first, and the last of a full gauge's.
	inline bool cap(int16_t x)
	{
		return x == START || x == WIDTH - 3;
	}
}
