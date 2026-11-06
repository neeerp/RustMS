#pragma once

#include <cmath>
#include <cstdint>
#include <utility>

namespace ms::layer_move
{
	// A map layer's own motion: the keys on the first canvas of an obj or a back (v83 FUN_0063ef34).
	struct Keys
	{
		// moveType: 1 swings along x, 2 along y, 3 round an ellipse.
		int32_t type = 0;
		// moveW and moveH: how far either way.
		int32_t width = 100;
		int32_t height = 100;
		// moveP: the ms of one swing, the other way round where it is negative.
		int32_t period = 5000;
		// moveR: the ms of one turn of the canvas, the other way round where it is negative.
		int32_t spin = 0;
		// centerStart: it leaves from where it was placed, not from the end of its swing.
		bool centered = false;
	};

	// The keys on a layer's first canvas. Only a key that's missing takes its default.
	template <typename Node>
	Keys read(Node canvas)
	{
		Keys keys;
		keys.type = static_cast<int32_t>(canvas["moveType"].get_integer());
		keys.spin = static_cast<int32_t>(canvas["moveR"].get_integer());

		if (keys.type == 0 && keys.spin == 0)
			return keys;

		if (auto width = canvas["moveW"])
			keys.width = static_cast<int32_t>(width.get_integer());

		if (auto height = canvas["moveH"])
			keys.height = static_cast<int32_t>(height.get_integer());

		if (auto period = canvas["moveP"])
			keys.period = static_cast<int32_t>(period.get_integer());

		keys.centered = canvas["centerStart"].get_integer() != 0;

		return keys;
	}

	// The degrees a Shape2D vector has turned, one turn every `period` ms (0x51408b08).
	inline double turned(int64_t elapsed, int32_t period)
	{
		if (period == 0)
			return 0.0;

		return static_cast<double>(elapsed % period) * 360.0 / period;
	}

	// The swing's offset from where the layer was placed, `elapsed` ms after it started. v83 builds it out of a
	// vector at (x, y) turning about the layer and another that copies its movement, each scaled (FUN_009d3009):
	// a vector's turn is rounded to whole px (0x51406fe7), and the scaling is integer division (0x5140847d).
	inline std::pair<int32_t, int32_t> offset(const Keys &keys, int64_t elapsed)
	{
		if (keys.type < 1 || keys.type > 3)
			return { 0, 0 };

		int32_t width = keys.type == 2 ? 0 : keys.width;
		int32_t height = keys.type == 1 ? 0 : keys.height;

		if (width == 0 && height == 0)
			return { 0, 0 };

		bool centered = keys.type != 3 && keys.centered;
		double radians = turned(elapsed, keys.period) * 0.017453292519943295;
		double cosine = std::cos(radians);
		double sine = std::sin(radians);
		auto turn = [&](int32_t x, int32_t y) {
			return std::pair<int32_t, int32_t>(static_cast<int32_t>(std::lround(x * cosine - y * sine)),
											   static_cast<int32_t>(std::lround(x * sine + y * cosine)));
		};

		if (width != 0)
		{
			int32_t start = centered ? -width : 0;
			auto [x, y] = centered ? turn(0, -width) : turn(width, 0);

			return { x, (y - start) * height / width };
		}

		auto [x, y] = centered ? turn(-height, 0) : turn(0, height);

		return { 0, y };
	}

	// The radians its canvas has turned, `elapsed` ms after it started (FUN_009d37a0).
	inline float spun(const Keys &keys, int64_t elapsed)
	{
		return static_cast<float>(turned(elapsed, keys.spin) * 0.017453292519943295);
	}
}
