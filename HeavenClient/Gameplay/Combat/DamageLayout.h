#pragma once

#include <cstdint>
#include <vector>

namespace ms
{
	// How a v83 damage number is laid out and animated (MapleLegends client:
	// Effect_HP FUN_00429685, the miss FUN_0042a3fc, and the mob's caller
	// FUN_005ca219). The digits are copied onto one canvas, 57 px tall, whose
	// top-left is 47 px above and half its width left of the anchor. It stays
	// opaque for 400 ms, fades out over 600 ms, and rises 30 px over the whole
	// second.
	namespace damage_layout
	{
		constexpr int16_t HEIGHT = 57;
		constexpr int16_t TOP = 47;
		constexpr int16_t RISE = 30;
		// A critical number's first digit starts this far right, past the
		// start of the NoCri1 "effect" banner, which sits at the top-left.
		constexpr int16_t CRITICAL_INDENT = 30;
		constexpr int32_t OPAQUE_TIME = 400;
		constexpr int32_t FADE_TIME = 600;
		constexpr int32_t DURATION = OPAQUE_TIME + FADE_TIME;

		struct Digit
		{
			int16_t width;
			int16_t height;
			// The image's origin x.
			int16_t origin;
		};

		struct Placed
		{
			// The digit's top-left on the canvas.
			int16_t left;
			int16_t top;
		};

		// Places `digits` on the canvas and returns its width. Each digit
		// overlaps the previous one by 3/5 of the part of it right of its
		// origin. Even digits (from 0) sit 3 px higher than odd ones, and
		// each `drops[i]` (0 or 1, rand() % 2 in v83) lowers digit i by 1 px.
		inline int16_t place(const std::vector<Digit>& digits, const std::vector<int16_t>& drops, bool critical,
			std::vector<Placed>& placed)
		{
			int16_t right = critical ? CRITICAL_INDENT : 0;
			int16_t overlap = 0;
			placed.clear();

			for (size_t i = 0; i < digits.size(); i++)
			{
				const Digit& digit = digits[i];
				int16_t left = right - overlap;
				int16_t bottom = HEIGHT - ((i % 2) ? 1 : 4) - drops[i];

				placed.push_back({ left, static_cast<int16_t>(bottom - digit.height) });
				right = left + digit.width;
				overlap = (digit.width - digit.origin) * 3 / 5;
			}

			return right;
		}

		// The left of a canvas (or of the "Miss" image, whose bottom is on
		// the anchor) of this width, relative to the anchor.
		inline int16_t left(int16_t width)
		{
			return -(width / 2);
		}

		inline float opacity(int32_t elapsed)
		{
			if (elapsed <= OPAQUE_TIME)
				return 1.0f;

			if (elapsed >= DURATION)
				return 0.0f;

			return 1.0f - static_cast<float>(elapsed - OPAQUE_TIME) / FADE_TIME;
		}

		inline float rise(int32_t elapsed)
		{
			if (elapsed >= DURATION)
				return RISE;

			return static_cast<float>(RISE) * elapsed / DURATION;
		}

		// How far above a mob's head the number for hit `line` (from 0) is
		// anchored: 15 px, plus 30 px a line, or 15 px for Demolition.
		inline int16_t mob_offset(int16_t line, bool close)
		{
			return 15 + line * (close ? 15 : 30);
		}
	}
}
