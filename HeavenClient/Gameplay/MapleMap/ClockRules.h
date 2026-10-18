#pragma once

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
}
