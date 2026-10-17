#pragma once

#include "ShotPath.h"

#include "../../Template/Rectangle.h"

#include <cstdint>
#include <optional>
#include <vector>

namespace ms
{
	// What a few shots rain on the field once they hit something, as in the
	// MapleRoyals client: Arrow Rain's arrows over the user, Inferno's and
	// Blizzard's flames and ice over where the shot hits, with Inferno's
	// fire on the ground there, and 3211004's over each mob it hits
	// (0x95496f, 0x9821f7). Each rain drops a few sprites at once, every so
	// often, from random spots in its area (FUN_0043612f, 0x441b30).
	namespace shot_rain
	{
		enum class Kind
		{
			NONE,
			OVER_USER,
			OVER_HIT,
			OVER_MOBS
		};

		inline Kind kind(int32_t skill)
		{
			switch (skill)
			{
				case 3111004: // Arrow Rain
				case 13111000:
					return Kind::OVER_USER;
				case 3111003: // Inferno
				case 3211003: // Blizzard
					return Kind::OVER_HIT;
				case 3211004:
					return Kind::OVER_MOBS;
				default:
					return Kind::NONE;
			}
		}

		// Whether the skill also sets the ground under its rain on fire
		// (FUN_00436ce9).
		inline bool burns(int32_t skill)
		{
			return skill == 3111003;
		}

		// How a skill's rain falls, from its `special` node: each sprite
		// moves (x, y), the way the user faces, over about `fall` ms;
		// `count` of them drop every `interval` ms, from `start` until
		// `duration` ms after the hit.
		struct Params
		{
			int16_t x = 0;
			int16_t y = 0;
			int32_t count = 0;
			int32_t fall = 0;
			int32_t interval = 0;
			int32_t start = 0;
			int32_t duration = 0;
		};

		// Arrow Rain's area: its lt/rb, given facing left, over the user and
		// 250 px higher (0x954a79).
		inline Rectangle<int16_t> over_user(Rectangle<int16_t> lt_rb, Point<int16_t> user, bool toleft)
		{
			int16_t left = toleft ? lt_rb.left() : -lt_rb.right();
			int16_t right = toleft ? lt_rb.right() : -lt_rb.left();

			return {
				static_cast<int16_t>(user.x() + left),
				static_cast<int16_t>(user.x() + right),
				static_cast<int16_t>(user.y() + lt_rb.top() - 250),
				static_cast<int16_t>(user.y() + lt_rb.bottom() - 250)
			};
		}

		// Inferno's and Blizzard's area: their lt/rb around where the shot
		// hits, unmirrored. Inferno's fire covers all of it (0x954b3b).
		inline Rectangle<int16_t> around(Rectangle<int16_t> lt_rb, Point<int16_t> hit)
		{
			return {
				static_cast<int16_t>(hit.x() + lt_rb.left()),
				static_cast<int16_t>(hit.x() + lt_rb.right()),
				static_cast<int16_t>(hit.y() + lt_rb.top()),
				static_cast<int16_t>(hit.y() + lt_rb.bottom())
			};
		}

		// Their rain falls from higher up: its top 50 px, its bottom 170.
		inline Rectangle<int16_t> raised(Rectangle<int16_t> area)
		{
			return {
				area.left(),
				area.right(),
				static_cast<int16_t>(area.top() - 50),
				static_cast<int16_t>(area.bottom() - 170)
			};
		}

		// 3211004's area on a mob: a strip 10 px tall at its feet, widened
		// to 150 px around its middle if it's narrower (0x9549d1, 0x982261).
		inline Rectangle<int16_t> over_mob(Rectangle<int16_t> body)
		{
			int32_t left = body.left();
			int32_t right = body.right();
			int32_t width = right - left;

			if (width < 150)
			{
				left += width / 2 - 75;
				right += 75 - width / 2;
			}

			return {
				static_cast<int16_t>(left),
				static_cast<int16_t>(right),
				static_cast<int16_t>(body.bottom() - 10),
				body.bottom()
			};
		}

		// When a rain drops its sprites.
		class Schedule
		{
		public:
			// Its first sprites drop `start` ms after the hit, and its last
			// no later than `duration` ms after it.
			Schedule(const Params &params, int64_t hit) : next(hit + params.start), end(hit + params.duration),
														  interval(params.interval)
			{}

			// Whether the rain is over by `now`.
			bool over(int64_t now) const
			{
				return now > end;
			}

			// Whether sprites drop at `now`. A late update drops them once,
			// and the next ones an interval after these were due.
			bool drop(int64_t now)
			{
				if (now > end || now < next)
					return false;

				next += interval;

				return true;
			}

		private:
			int64_t next;
			int64_t end;
			int32_t interval;
		};

		// How far a sprite falls. It slants the way the user faces, and
		// falls a little further or shorter by `r`, random in [0, 40].
		inline Point<int16_t> fall(const Params &params, bool toleft, int32_t r)
		{
			int32_t dy = params.y;

			if (params.x != 0 && params.y != 0)
				dy = (20 - r) * params.x / params.y + params.y;

			return { static_cast<int16_t>(toleft ? -params.x : params.x), static_cast<int16_t>(dy) };
		}

		// How long it falls for, with `r` random in [0, 299]. It's gone
		// when it lands.
		inline int32_t flight(const Params &params, int32_t r)
		{
			return params.fall - 150 + r;
		}

		// How long a sprite takes to fade from opaque to its own opacity,
		// (128 + r) / 255 with `r` random in [0, 63].
		constexpr int32_t FADE = 150;

		inline float opacity(int32_t r)
		{
			return (128 + r) / 255.0f;
		}

		// Inferno's fire: a flame every `distance` px across its area
		// (FUN_0043e357).
		inline std::vector<int16_t> columns(Rectangle<int16_t> area, int16_t distance)
		{
			std::vector<int16_t> xs;

			if (distance <= 0)
				return xs;

			for (int32_t x = area.left(); x < area.right(); x += distance)
				xs.push_back(static_cast<int16_t>(x));

			return xs;
		}

		// Where a platform that runs left to right crosses `x`, if it's
		// within the area's height (FUN_00a456f9).
		inline std::optional<int16_t> ground(const shot_path::Line &platform, int16_t x, Rectangle<int16_t> area)
		{
			if (platform.x1 >= platform.x2 || x < platform.x1 || x > platform.x2)
				return std::nullopt;

			int32_t y = (platform.y2 - platform.y1) * (x - platform.x1) / (platform.x2 - platform.x1) + platform.y1;

			if (y < area.top() || y > area.bottom())
				return std::nullopt;

			return static_cast<int16_t>(y);
		}

		// The flames sit within 2 px of the ground, by `r` in [0, 4]. Each
		// lights from the hit, fading in over (k + 1) * 100 ms, k random in
		// [0, 4], to an opacity of (128 + r) / 255, r random in [0, 127].
		// They all burn for BURN ms, then fade out over as long (0x442887).
		constexpr int32_t BURN = 500;

		inline int16_t flame_y(int16_t ground, int32_t r)
		{
			return static_cast<int16_t>(ground + r - 2);
		}

		inline int32_t flame_fade(int32_t k)
		{
			return (k + 1) * 100;
		}

		// How opaque something fading from `from` at `start` to `to` at
		// `end` is at `now`.
		inline float fade(float from, float to, int64_t start, int64_t end, int64_t now)
		{
			if (now <= start)
				return from;

			if (now >= end)
				return to;

			return from + (to - from) * static_cast<float>(now - start) / static_cast<float>(end - start);
		}
	}
}
