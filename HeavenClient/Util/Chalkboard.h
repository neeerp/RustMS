#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// How v83 lays out a chalkboard's note (FUN_004890c4 with the
	// ChatBalloon.img "adboard" style).
	namespace Chalkboard
	{
		// The longest note another player's board shows.
		constexpr size_t MAX_TEXT = 40;
		// A line is at most 14 tiles of 14 px.
		constexpr int16_t TILE = 14;
		constexpr int16_t MAX_TILES = 14;
		constexpr int16_t MAX_WIDTH = TILE * MAX_TILES;

		inline const char *const BLANKS = " \t\r\n";

		inline std::string trim(const std::string &text)
		{
			size_t first = text.find_first_not_of(BLANKS);

			if (first == std::string::npos)
				return "";

			return text.substr(first, text.find_last_not_of(BLANKS) - first + 1);
		}

		// Another player's note is trimmed, has its curse words masked, and is
		// cut to 40 bytes. The player's own note is only trimmed.
		template <typename Filter>
		std::string remote_text(const std::string &text, Filter filter)
		{
			std::string shown = filter(trim(text));

			if (shown.size() > MAX_TEXT)
				shown.resize(MAX_TEXT);

			return shown;
		}

		// Wraps by character: each line takes as many characters as fit, loses
		// its trailing blanks, and the next line skips its leading ones.
		template <typename Width>
		std::vector<std::string> wrap(const std::string &text, Width width, int16_t max_width = MAX_WIDTH)
		{
			std::vector<std::string> lines;
			size_t start = text.find_first_not_of(BLANKS);

			while (start != std::string::npos)
			{
				size_t count = 1;

				while (start + count < text.size() && width(text.substr(start, count + 1)) <= max_width)
					count++;

				std::string line = text.substr(start, count);
				line.erase(line.find_last_not_of(BLANKS) + 1);
				lines.push_back(line);

				start = text.find_first_not_of(BLANKS, start + count);
			}

			return lines;
		}

		// A single line gets an odd number of tiles, so the arrow under the
		// middle tile is centred. Longer notes use the full width.
		inline int16_t tiles(size_t lines, int16_t first_width)
		{
			if (lines > 1)
				return MAX_TILES;

			return static_cast<int16_t>((first_width + TILE - 1) / TILE) | 1;
		}
	}
}
