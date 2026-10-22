#pragma once

#include <cmath>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// Kites (cash items 508xxxx): messages players fly over a map, as v83
	// shows them (SPAWN_KITE, FUN_0065acdf).
	namespace kite
	{
		// The lines a kite carries, and the most characters in each.
		constexpr size_t LINES = 3;
		constexpr size_t MAX_LINE = 60;

		// The ms a kite takes to fade out (REMOVE_KITE type 0, FUN_0065b2ca).
		constexpr int16_t FADE = 1000;

		// The art hangs from a point this far above and left of the flier's
		// feet (FUN_0065acdf), the middle of the 60x80 box that takes the
		// mouse (FUN_0065aa30), which doesn't bob with it.
		constexpr int16_t RISE = 100;
		constexpr int16_t SHIFT = -3;
		constexpr int16_t HALF_WIDTH = 30;
		constexpr int16_t HALF_HEIGHT = 40;

		// The bob: 3 px either way, a step of 0.0942477 rad each 30 ms
		// update (FUN_0065ab00), so once every 2 s, starting from a random
		// rand() % 360.
		constexpr double BOB = 3.0;
		constexpr double BOB_STEP = 0.0942477;
		constexpr double BOB_MS = 30.0;

		// v83 refuses a kite in the Free Market's rooms (910xxxxxx) before
		// asking for its lines.
		inline bool allowed_on(int32_t mapid)
		{
			return mapid / 10000000 != 91;
		}

		// The entry window's three lines as v83 sends them: always all
		// three, empty ones kept.
		inline std::string join(const std::vector<std::string> &lines)
		{
			std::string message;

			for (size_t i = 0; i < LINES; i++)
			{
				if (i > 0)
					message += '\n';

				if (i < lines.size())
					message += lines[i];
			}

			return message;
		}

		// A kite's message as the viewer lays it out, a line per row.
		inline std::vector<std::string> split(const std::string &message)
		{
			std::vector<std::string> lines;
			size_t start = 0;

			while (lines.size() < LINES)
			{
				size_t end = message.find('\n', start);

				if (end == std::string::npos)
				{
					lines.push_back(message.substr(start));
					break;
				}

				lines.push_back(message.substr(start, end - start));
				start = end + 1;
			}

			return lines;
		}

		// What the mouse over a kite shows (FUN_0094cfc3): its flier's name
		// and the item's `msg`, run together as v83 does ("Nameis wishing
		// you a happy new year.").
		inline std::string hover_text(const std::string &name, const std::string &msg)
		{
			return name + msg;
		}

		// Whether the mouse at (mx, my) is on the kite flown at (x, y), in
		// map coordinates.
		inline bool hit(int16_t x, int16_t y, int32_t mx, int32_t my)
		{
			int32_t cx = x + SHIFT;
			int32_t cy = y - RISE;

			return mx >= cx - HALF_WIDTH && mx <= cx + HALF_WIDTH && my >= cy - HALF_HEIGHT &&
				   my <= cy + HALF_HEIGHT;
		}

		// The art's vertical offset at `phase`.
		inline double bob(double phase)
		{
			return BOB * std::sin(phase);
		}

		// How opaque a fading kite is with `left` ms of its fade to go.
		inline float fade_opacity(int16_t left)
		{
			if (left <= 0)
				return 0.0f;

			return left >= FADE ? 1.0f : static_cast<float>(left) / FADE;
		}
	}
}
