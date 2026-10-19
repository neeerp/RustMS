#pragma once

#include "../../Graphics/Animation.h"

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

// How v83 plays a map object's animation, and switches a named object between its states.
namespace ms::obj_state
{
	struct Playback
	{
		// Whether it animates at all.
		bool animates;
		Animation::End end;
		int16_t loop_from;
		// Whether one still going carries on rather than starting again.
		bool waits;
	};

	// An object's `repeat` (FUN_006431e3). From 0 up it loops back to that frame (Animate 0x20); -1 plays once and
	// stays on the last frame (0); -2 plays once and returns to the first (0x10), but only if it isn't going already.
	// Below that it stays on its first frame. The flags' meanings are read from the data, not from Gr2D.
	inline Playback playback(int32_t repeat)
	{
		if (repeat >= 0)
			return { true, Animation::End::LOOP, static_cast<int16_t>(repeat), false };

		if (repeat == -1)
			return { true, Animation::End::HOLD, 0, false };

		if (repeat == -2)
			return { true, Animation::End::FIRST, 0, true };

		return { false, Animation::End::LOOP, 0, false };
	}

	// The state a named object shows after a switch to `state`, of `count` (FUN_00642aca). -1 shows the current one
	// again. A state it doesn't have changes nothing, which is -1 here.
	inline int32_t next_state(int32_t current, int32_t count, int32_t state)
	{
		if (state == -1)
			return current;

		if (state < 0 || state >= count)
			return -1;

		return state;
	}

	// An obj's `tags`, which split on "," with the spaces around each trimmed (FUN_0063ad16); ";" doesn't split.
	inline std::vector<std::string> split_tags(const std::string &tags)
	{
		std::vector<std::string> split;
		size_t start = 0;

		while (start <= tags.size())
		{
			size_t end = std::min(tags.find(',', start), tags.size());
			std::string piece = tags.substr(start, end - start);
			size_t first = piece.find_first_not_of(" \t\r\n");

			if (first != std::string::npos)
				split.push_back(piece.substr(first, piece.find_last_not_of(" \t\r\n") - first + 1));

			start = end + 1;
		}

		return split;
	}
}
