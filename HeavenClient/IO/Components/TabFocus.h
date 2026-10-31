#pragma once

#include <cstddef>
#include <optional>

namespace ms
{
	// v83's dialogs move the keys with Tab (FUN_004edc29, report 179e):
	// to the next control in the order they were made, or the one before
	// with Shift, wrapping at both ends, past any that is disabled or takes
	// no focus, and nowhere after going all the way round.
	namespace tab_focus
	{
		// The stop after `from` of `count` that `accepts` takes, or nothing.
		// From nowhere, Tab starts at the first and Shift+Tab at the last.
		template<class Accepts>
		std::optional<size_t> next(std::optional<size_t> from, size_t count, bool back, Accepts accepts)
		{
			if (count < 2 && from)
				return std::nullopt;

			size_t at = from ? *from : (back ? 0 : count - 1);

			for (size_t step = 0; step < count; step++)
			{
				at = back ? (at + count - 1) % count : (at + 1) % count;

				if (from && at == *from)
					return std::nullopt;

				if (accepts(at))
					return at;
			}

			return std::nullopt;
		}
	}
}
