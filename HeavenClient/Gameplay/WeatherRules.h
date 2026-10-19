#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <utility>

// The numbers behind v83's weather: BLOW_WEATHER (0x8E), the 512xxxx cash items and a map's weather node
// (CField::BlowWeather FUN_00534aca).
namespace ms::weather
{
	// Only 512xxxx items blow a weather (cash type 0x10 in FUN_004863d5).
	inline bool is_weather(int32_t item_id)
	{
		return item_id / 10000 == 512;
	}

	// The message bar stays 20 s. A map's weather, or an admin's whisper shown as one, stops itself 22 s after it
	// starts; a packet's lasts until the server stops it.
	constexpr int32_t BAR_TIME = 20000;
	constexpr int32_t MAP_TIME = 22000;

	// An admin's whisper (flag 0x92) shows as this item's weather: a message bar and nothing else.
	constexpr int32_t WHISPER_ITEM = 5120025;

	// The bar's art is drawn at 192 of 255, its top 160 px down the screen (FUN_0058c53b).
	constexpr float BAR_ALPHA = 192.0f / 255.0f;
	constexpr int16_t BAR_TOP = 160;

	struct Bar
	{
		int16_t left;
		int16_t width;
		int16_t tiles;
		int16_t text_x;
	};

	// The bar is a left cap, as many middle tiles as the text needs and a right cap, centred on the view. The text
	// is centred over the tiles.
	inline Bar bar(int16_t view_width, int16_t left_width, int16_t tile_width, int16_t right_width, int16_t text_width)
	{
		int16_t tiles = tile_width > 0 ? static_cast<int16_t>((text_width + tile_width - 1) / tile_width) : 0;
		int16_t width = static_cast<int16_t>(left_width + tiles * tile_width + right_width);
		int16_t left = static_cast<int16_t>((view_width - width) / 2);
		int16_t text_x = static_cast<int16_t>(left_width + (tiles * tile_width - text_width) / 2);

		return { left, width, tiles, text_x };
	}

	// A `random` node without a count has 100, and either way only 30 to 54 % of them fall. `r` is a rand().
	inline int32_t particle_count(bool random, int32_t count, int32_t r)
	{
		return random ? count * (r % 25 + 30) / 100 : count;
	}

	// One falling particle of types 1 and 2 (FUN_0063fe57).
	struct Particle
	{
		// Its drawn width and height when the node resizes, 15 to 24.
		int32_t size;
		// Where its sway starts, in degrees.
		int32_t phase;
		// The ms it takes to fade in.
		int32_t fade;
		// Which of the node's canvases, or of its animations for an `ani` node.
		int32_t frame;
		int32_t sx;
		int32_t sy;
		int32_t ex;
		int32_t ey;
		// The ms per turn of a type 2 particle, 0 for none.
		int32_t spin;
		// The ms it takes to move from its start to its target, 0 for a particle that stays.
		int32_t period;
	};

	// Directions 4 and 5 fly across the screen from its bottom or its centre; the others move in the world,
	// wrapping around the view.
	inline bool on_screen(int32_t direction)
	{
		return direction == 4 || direction == 5;
	}

	// Draw a particle as the client does. `next()` is the C library's rand(), drawn in the client's order: the
	// size, the phase, the fade, the canvas, the start and the target, the animation, the spin and, flying across
	// the screen, the time. The phase goes to cos and sin as radians, as v83 has it.
	template<class Next>
	Particle particle(int32_t type, int32_t direction, int32_t speed, int32_t canvases, int32_t animations, Next next)
	{
		Particle p{};

		int32_t first = next() % 4;
		p.size = first * (next() % 4) + 15;
		p.phase = next() % 360;
		p.fade = next() % 3000 + 3000;

		if (animations == 0 && canvases > 0)
			p.frame = static_cast<int32_t>(static_cast<uint32_t>(next()) % static_cast<uint32_t>(canvases));

		switch (direction)
		{
			case 1:
				p.sx = next() % 800 - 400;
				p.sy = 300 - next() % 600;
				p.ex = p.sx + next() % 200 - 100;
				p.ey = p.sy - 300;
				break;
			case 2:
				p.sx = 400 - next() % 800;
				p.sy = next() % 600 - 300;
				p.ex = p.sx - 400;
				p.ey = p.sy + next() % 200 - 100;
				break;
			case 3:
				p.sx = next() % 800 - 400;
				p.sy = next() % 600 - 300;
				p.ex = p.sx + 400;
				p.ey = p.sy + next() % 200 - 100;
				break;
			case 4:
				p.sx = next() % 20 + 390;
				p.sy = next() % 20 + 590;
				p.ex = static_cast<int32_t>(std::cos(static_cast<double>(p.phase)) * 721.0 + 400.0);
				// v83 truncates the sine before scaling it, so this is nearly always 0.
				p.ey = std::abs(static_cast<int32_t>(std::sin(static_cast<double>(p.phase)))) * 721;
				break;
			case 5:
				p.sx = next() % 20 + 390;
				p.sy = next() % 20 + 290;
				p.ex = static_cast<int32_t>(std::cos(static_cast<double>(p.phase)) * 500.0 + 400.0);
				p.ey = static_cast<int32_t>(std::sin(static_cast<double>(p.phase)) * 500.0 + 300.0);
				break;
			case 0:
				p.sx = next() % 800 - 400;
				p.sy = next() % 600 - 300;
				p.ex = p.sx + next() % 200 - 100;
				p.ey = p.sy + 300;
				break;
			case 6:
				// Direction 6 stays where it starts.
				p.sx = next() % 800 - 400;
				p.sy = next() % 600 - 300;
				p.ex = p.sx;
				p.ey = p.sy;
				break;
			default:
				// v83 keeps the last particle's spot for any other direction. No item has one.
				break;
		}

		if (animations > 0)
			p.frame = next() % animations;

		if (type == 2)
			p.spin = next() % 3000 + 300;

		int32_t base = 50000 - 8000 * speed;

		if (on_screen(direction))
			p.period = (next() % 30000 + base - 15000) / p.size;
		else if (direction >= 0 && direction <= 3)
			p.period = base / p.size;

		return p;
	}

	// How far a particle has moved along `distance` after `elapsed` ms, at one distance per period, forever.
	inline int32_t moved(int32_t distance, int64_t elapsed, int32_t period)
	{
		if (period <= 0)
			return 0;

		return static_cast<int32_t>(distance * (elapsed / period) + distance * (elapsed % period) / period);
	}

	// Bring `value` into [low, low + width).
	inline int32_t wrap(int32_t value, int32_t low, int32_t width)
	{
		if (width <= 0)
			return low;

		int32_t offset = (value - low) % width;

		if (offset < 0)
			offset += width;

		return low + offset;
	}

	struct Spot
	{
		int32_t x;
		int32_t y;
	};

	// The centre of a particle moving in the world, which wraps around the view centred on (centre_x, centre_y).
	// It sways in a circle of half its size every 2 s, `sway` % of it across and 5 % up and down.
	inline Spot world_spot(const Particle &p, int64_t elapsed, int32_t centre_x, int32_t centre_y, int32_t sway,
						   int16_t view_width, int16_t view_height)
	{
		int32_t x = p.sx + moved(p.ex - p.sx, elapsed, p.period);
		int32_t y = p.sy + moved(p.ey - p.sy, elapsed, p.period);
		x = wrap(x, centre_x - view_width / 2 - p.size, view_width + p.size);
		y = wrap(y, centre_y - view_height / 2 - p.size, view_height + p.size);

		constexpr double DEGREES = 3.14159265358979323846 / 180.0;
		double start = p.phase * DEGREES;
		double now = (p.phase + 360.0 * static_cast<double>(elapsed % 2000) / 2000.0) * DEGREES;
		// The circle's radius is half the size, truncated.
		auto radius = static_cast<double>(p.size / 2);
		x += static_cast<int32_t>(radius * (std::cos(now) - std::cos(start)) * sway / 100);
		y += static_cast<int32_t>(radius * (std::sin(now) - std::sin(start)) * 5 / 100);

		return { x, y };
	}

	// The centre of a particle flying across the 800x600 screen. It starts again when it reaches its target.
	inline Spot screen_spot(const Particle &p, int64_t elapsed)
	{
		int32_t x = p.sx + moved(p.ex - p.sx, elapsed, p.period);
		int32_t y = p.sy + moved(p.ey - p.sy, elapsed, p.period);
		x = wrap(x, std::min(p.sx, p.ex) + (p.ex < p.sx ? 1 : 0), std::abs(p.ex - p.sx));
		y = wrap(y, std::min(p.sy, p.ey) + (p.ey < p.sy ? 1 : 0), std::abs(p.ey - p.sy));

		return { x, y };
	}

	// A type 2 particle turns once per spin, in degrees.
	inline double spin_angle(const Particle &p, int64_t elapsed)
	{
		if (p.spin <= 0)
			return 0.0;

		return 360.0 * static_cast<double>(elapsed % p.spin) / p.spin;
	}

	// A particle fades in from 1 to 255 less twice its size over its fade. An animated one shows at once
	// (I: v83 reads an unset alpha there).
	inline float fade_in(const Particle &p, int64_t elapsed, bool animated)
	{
		if (animated)
			return 255.0f;

		float target = p.size > 100 ? 50.0f : 255.0f - 2.0f * p.size;

		if (elapsed >= p.fade)
			return target;

		return 1.0f + (target - 1.0f) * static_cast<float>(elapsed) / p.fade;
	}

	// When the weather stops, each particle fades from where it was to nothing over its own 1000 to 1999 ms
	// (FUN_00641c28). `r` is a rand().
	inline int32_t fade_out_time(int32_t r)
	{
		return r % 1000 + 1000;
	}

	inline float fade_out(float from, int64_t elapsed, int32_t length)
	{
		if (elapsed >= length)
			return 0.0f;

		return from * (1.0f - static_cast<float>(elapsed) / length);
	}

	// Type 3 bursts its node's five animations (path0 to path4) for 30 s, each every `interval` ms, `count` at a
	// time (FUN_006419e7).
	constexpr int32_t SHOW_TIME = 30000;
	constexpr std::array<std::pair<int32_t, int32_t>, 5> BURSTS = { {
		{ 200, 1 }, { 1000, 5 }, { 500, 3 }, { 300, 2 }, { 200, 1 }
	} };

	// The part of the 800x600 screen a burst has yet to spread over. It starts at five sixths and shrinks by a
	// quarter at every volley (FUN_00442184).
	struct Spread
	{
		int32_t width = 800 * 5 / 6;
		int32_t height = 600 * 5 / 6;
	};

	// The box the next volley lands in, centred on the screen, which grows each time.
	inline std::pair<int32_t, int32_t> spread(Spread &s)
	{
		int32_t x = 800 - s.width;
		int32_t y = 600 - s.height;
		s.width = s.width * 3 / 4;
		s.height = s.height * 3 / 4;

		return { x, y };
	}

	// Where in a box `range` wide, centred on the screen, a burst lands. `r` is a random number.
	inline int32_t burst_offset(int32_t range, uint32_t r)
	{
		return static_cast<int32_t>(r % static_cast<uint32_t>(range)) - range / 2;
	}

	// Type 4 sets off three fireworks every 2 s for 30 s, this far from the screen's centre, with a looping sound
	// (FUN_00442487).
	constexpr int32_t FIREWORK_INTERVAL = 2000;
	constexpr std::array<int16_t, 3> FIREWORK_X = { -230, 0, 230 };
	constexpr int16_t FIREWORK_Y = -60;

	// Every frame of a weather's animation without a delay of its own lasts 120 ms (FUN_0043f768).
	constexpr uint16_t FRAME_DELAY = 120;
}
