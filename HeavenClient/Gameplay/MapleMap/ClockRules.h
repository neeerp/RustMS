#pragma once

#include <array>
#include <cstdint>

namespace ms
{
	// The wall clock a map's `clock` node puts on a station's sign, which
	// CLOCK (147) kind 1 sets, as v83 keeps it (0x49ae84, 0x49ad14).
	namespace wall_clock
	{
		// What the clock shows: hours on a 12 hour dial, though a long stay
		// can carry them past 12 (v83 never flips am and pm).
		struct Time
		{
			bool pm = false;
			int32_t hour = 0;
			int32_t minute = 0;
			int32_t second = 0;

			bool operator==(const Time &other) const
			{
				return pm == other.pm && hour == other.hour && minute == other.minute && second == other.second;
			}
		};

		// The time CLOCK sends, on a 24 hour dial: midnight is "AM 00" and
		// noon "PM 12".
		inline Time set(int32_t hour, int32_t minute, int32_t second)
		{
			Time time;
			time.pm = hour / 12 != 0;
			time.hour = hour % 12;

			if (time.pm && time.hour == 0)
				time.hour = 12;

			time.minute = minute;
			time.second = second;
			return time;
		}

		// `seconds` after `base` (0x49ae19). The hour wraps at 24 and the
		// am or pm stays as it was.
		inline Time after(Time base, int32_t seconds)
		{
			int32_t total = base.second + seconds;
			int32_t minute = base.minute + total / 60;

			Time time;
			time.pm = base.pm;
			time.second = total % 60;
			time.hour = (base.hour + minute / 60) % 24;
			time.minute = minute % 60;
			return time;
		}

		// The colon blinks: solid on even seconds, faint on odd ones.
		inline float colon_opacity(int32_t second)
		{
			return second % 2 != 0 ? 64.0f / 255.0f : 1.0f;
		}

		// Where each glyph's top left goes in the clock's box.
		struct Layout
		{
			int32_t tag;
			int32_t hour_tens;
			int32_t hour_ones;
			int32_t colon;
			int32_t minute_tens;
			int32_t minute_ones;
			int32_t y;
		};

		// The glyphs centred in a `width` by `height` box, the am or pm tag
		// first and a gap the width of the colon after it (0x49a8d8).
		inline Layout layout(int32_t width, int32_t height, int32_t digit_width, int32_t digit_height,
							 int32_t colon_width, int32_t tag_width)
		{
			Layout layout;
			layout.tag = (width - (2 * colon_width + 4 * digit_width + tag_width + 8)) >> 1;
			layout.hour_tens = layout.tag + colon_width + tag_width;
			layout.hour_ones = layout.hour_tens + digit_width + 2;
			layout.colon = layout.hour_ones + digit_width + 2;
			layout.minute_tens = layout.colon + colon_width + 2;
			layout.minute_ones = layout.minute_tens + digit_width + 2;
			layout.y = (height - digit_height) >> 1;
			return layout;
		}

		// The clock between settings. Until the first it shows "AM 00:00"
		// and stands still; after one, it catches up with the time a second
		// or more at a time, the first a second after the setting.
		class Clock
		{
		public:
			void set(Time time)
			{
				base = time;
				valid = true;
				since_set = 0;
				since_refresh = 0;
			}

			void update(int32_t ms)
			{
				since_set += ms;
				since_refresh += ms;

				if (since_refresh < 1000)
					return;

				since_refresh = 0;

				if (valid)
					shown = after(base, since_set / 1000);
			}

			const Time &get() const
			{
				return shown;
			}

		private:
			Time base;
			Time shown;
			bool valid = false;
			int32_t since_set = 0;
			int32_t since_refresh = 0;
		};
	}

	// The countdown CLOCK (147) kinds 0, 2 and 3 put at the top of the
	// screen, as v83 keeps it (0x49ae6a, 0x49aeee, 0x49ad14, 0x49a0f4).
	namespace countdown
	{
		// What a running clock reads with `left_ms` to go: whole seconds,
		// rounded toward zero, so a clock started at 10 reads 9 at once and
		// -1 a second after the end (0x49ad14).
		inline int32_t seconds_left(int64_t left_ms)
		{
			return static_cast<int32_t>(left_ms / 1000);
		}

		// What the clock shows for a reading: below an hour, minutes and
		// seconds; from an hour on, hours and minutes on the hour
		// background. A reading below 1 shows 0, and a first digit of 0 is
		// left out (0x49a0f4).
		struct Face
		{
			bool hours = false;
			// -1 for a digit left out.
			std::array<int32_t, 4> digits = {};
		};

		inline Face face(int32_t seconds)
		{
			if (seconds < 1)
				seconds = 0;

			Face face;
			face.hours = seconds >= 3600;

			int32_t high = face.hours ? seconds / 3600 : seconds / 60;
			int32_t low = face.hours ? seconds % 3600 / 60 : seconds % 60;
			int32_t first = high / 10 % 10;

			face.digits = { first != 0 ? first : -1, high % 10, low / 10, low % 10 };
			return face;
		}

		// Where each digit's top left goes in the clock's 258 by 58 box
		// (0x52bf42, 0x49a0f4).
		constexpr std::array<int16_t, 4> DIGIT_X = { 86, 112, 179, 205 };
		constexpr int16_t DIGIT_Y = 12;
		constexpr int16_t WIDTH = 258;
		// v83 puts the box at (271, 30), centred on its 800 wide screen
		// (0x536428).
		constexpr int16_t TOP = 30;

		// Whether an event's clock (kind 0) stays up on `mapid`: on event
		// maps, those with a 9 in the millions, other than their 5 in the
		// ten thousands (0x536575, from 0xa03350).
		inline bool stays_on(int32_t mapid)
		{
			return mapid / 1000000 % 100 == 9 && mapid / 10000 % 100 != 5;
		}
	}
}
