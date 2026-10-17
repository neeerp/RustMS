#pragma once

#include "../../Template/Rectangle.h"

#include <cstdint>

namespace ms
{
	// What a few spells leave over the field when cast, as in the
	// MapleRoyals client, whether they hit anything or not, for the caster
	// and for others alike (0x956b4e, 0x982b33): Big Bang's and Explosion's
	// blasts spreading out from the middle of their area, and Meteor
	// Shower's, Blizzard's and Genesis's fire, ice and light over the
	// ground in theirs.
	namespace spell_area
	{
		enum class Kind
		{
			NONE,
			BIG_BANG,
			EXPLOSION,
			TILES
		};

		inline Kind kind(int32_t skill)
		{
			switch (skill)
			{
				case 2121001: // Big Bang
				case 2221001:
				case 2321001:
				case 22121000:
				case 22151001:
					return Kind::BIG_BANG;
				case 2111002: // Explosion
					return Kind::EXPLOSION;
				case 2121007: // Meteor Shower
				case 2221007: // Blizzard
				case 2321008: // Genesis
				case 12111003:
					return Kind::TILES;
				default:
					return Kind::NONE;
			}
		}

		// Big Bang's blast grows with how long it was charged, in ms, 40 px
		// above the user (0x956404, 0x982c20).
		inline Rectangle<int16_t> big_bang(int32_t charge, Point<int16_t> user)
		{
			int32_t x = 100 + charge / 50;
			int32_t y = 75 + charge * 3 / 200;

			return {
				static_cast<int16_t>(user.x() - x),
				static_cast<int16_t>(user.x() + x),
				static_cast<int16_t>(user.y() - 40 - y),
				static_cast<int16_t>(user.y() - 40 + y)
			};
		}

		// Explosion's is its lt/rb around the user, also 40 px up
		// (0x956b91, 0x982b8a).
		inline Rectangle<int16_t> explosion(Rectangle<int16_t> lt_rb, Point<int16_t> user)
		{
			return {
				static_cast<int16_t>(user.x() + lt_rb.left()),
				static_cast<int16_t>(user.x() + lt_rb.right()),
				static_cast<int16_t>(user.y() - 40 + lt_rb.top()),
				static_cast<int16_t>(user.y() - 40 + lt_rb.bottom())
			};
		}

		// The tiles cover the skill's lt/rb, given facing left, around the
		// user (0x956c66).
		inline Rectangle<int16_t> tiles(Rectangle<int16_t> lt_rb, Point<int16_t> user, bool toleft)
		{
			int16_t left = toleft ? lt_rb.left() : -lt_rb.right();
			int16_t right = toleft ? lt_rb.right() : -lt_rb.left();

			return {
				static_cast<int16_t>(user.x() + left),
				static_cast<int16_t>(user.x() + right),
				static_cast<int16_t>(user.y() + lt_rb.top()),
				static_cast<int16_t>(user.y() + lt_rb.bottom())
			};
		}

		// The tiles light 180 ms after the cast and burn this long before
		// fading out.
		constexpr int32_t TILE_DELAY = 180;

		inline int32_t burn_time(int32_t skill)
		{
			return skill == 2221007 || skill == 2321008 ? 2760 : 3000;
		}

		// Where each wave of a blast lands: a band around the middle of its
		// area, a sixth of its size at first, a quarter more of what's left
		// each wave (FUN_004365db, FUN_00441f45).
		class Spread
		{
		public:
			Spread(Rectangle<int16_t> area) : x((area.left() + area.right()) / 2), y((area.top() + area.bottom()) / 2),
											  width(area.right() - area.left()), height(area.bottom() - area.top()),
											  inner_width(width * 5 / 6), inner_height(height * 5 / 6)
			{}

			// The band for the next wave.
			Rectangle<int16_t> widen()
			{
				int32_t w = width - inner_width;
				int32_t h = height - inner_height;
				inner_width = inner_width * 3 / 4;
				inner_height = inner_height * 3 / 4;

				return {
					static_cast<int16_t>(x - w / 2),
					static_cast<int16_t>(x - w / 2 + w),
					static_cast<int16_t>(y - h / 2),
					static_cast<int16_t>(y - h / 2 + h)
				};
			}

		private:
			int32_t x;
			int32_t y;
			int32_t width;
			int32_t height;
			int32_t inner_width;
			int32_t inner_height;
		};
	}
}
