#pragma once

#include "CloudLayout.h"

#include <cstdint>
#include <cstdlib>
#include <utility>
#include <vector>

// How v83 moves a map obj with `flow` and repeats it every cx/cy px (FUN_0063c212, FUN_0063e2c5,
// reports/201-obj-keys.md).
namespace ms::obj_flow
{
	// How far its (x, y) has moved `elapsed` ms after its map loaded. It starts 100 px behind and moves 100 px
	// every 20000/|r| ms for good, right or down where r is positive. Flow 1 moves along x at rx, 2 along y at
	// ry, and 3 along both at rx.
	inline std::pair<int64_t, int64_t> offset(int32_t flow, int32_t rx, int32_t ry, int64_t elapsed)
	{
		bool along_x = flow == 1 || flow == 3;
		bool along_y = flow == 2 || flow == 3;
		int32_t r = along_x ? rx : ry;
		int64_t period = r == 0 ? 0 : 20000 / std::abs(r);

		if (period == 0)
			return { 0, 0 };

		int64_t moved = (r > 0 ? 1 : -1) * (-100 + 100 * elapsed / period);

		return { along_x ? moved : 0, along_y ? moved : 0 };
	}

	// How far past its first copy the copies go, for a view `view` px wide and copies `period` px apart:
	// v83's unsigned (view + 2*period - 2) / period * period, which leaves none past a negative period.
	inline int32_t extent(int32_t view, int32_t period)
	{
		if (period == 0)
			return 0;

		uint32_t span = static_cast<uint32_t>(view + 2 * period - 2) / static_cast<uint32_t>(period);

		return static_cast<int32_t>(span) * period;
	}

	// Where the copies of an obj at (x, y) go, with the view `width` x `height` centred on (centre_x,
	// centre_y). Each axis it flows along wraps into a window that keeps the view covered.
	inline std::vector<std::pair<int32_t, int32_t>> copies(int32_t flow, int32_t cx, int32_t cy, int32_t width,
														  int32_t height, int64_t x, int64_t y, int32_t centre_x,
														  int32_t centre_y)
	{
		std::vector<std::pair<int32_t, int32_t>> result;
		int32_t across = flow & 1 ? extent(width, cx) : 0;
		int32_t down = flow & 2 ? extent(height, cy) : 0;

		if (across + cx <= 0 || down + cy <= 0)
			return result;

		for (int32_t iy = 0; iy < down + cy; iy += cy)
		{
			for (int32_t ix = 0; ix < across + cx; ix += cx)
			{
				int64_t px = x + ix;
				int64_t py = y + iy;

				if (flow & 1)
					px = map_clouds::wrap(px, centre_x - cx - across / 2, across + cx);

				if (flow & 2)
					py = map_clouds::wrap(py, centre_y - cy - down / 2, down + cy);

				result.emplace_back(static_cast<int32_t>(px), static_cast<int32_t>(py));
			}
		}

		return result;
	}
}
