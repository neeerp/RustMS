#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// OPEN_UI_STRING_LIST's (233) slideshow, the NPC dialog's type 9 (v83 FUN_00961275, report b098): one image per
	// page, turned by Prev and Next.
	namespace image_slides
	{
		// The pages from the packet's paths; empty ones are not added (0x96132b).
		inline std::vector<std::string> pages(const std::vector<std::string> &paths)
		{
			std::vector<std::string> kept;

			for (const std::string &path : paths)
				if (!path.empty())
					kept.push_back(path);

			return kept;
		}

		// Prev and Next (dlg+0xb4, +0xb8). OK shows where there is no Next and Close always (FUN_009aa346).
		struct Buttons
		{
			bool prev;
			bool next;

			bool ok() const
			{
				return !next;
			}
		};

		// On opening, Next goes by the packet's count, empty paths included (0x961367).
		inline Buttons opened(size_t count)
		{
			return { false, count > 1 };
		}

		// After Prev or Next moves to `page` (0x9ab9c3): Next against the pages kept, and left as it was past the
		// last one, which only empty paths let happen.
		inline Buttons turned(Buttons before, uint16_t page, size_t pages)
		{
			uint32_t last = static_cast<uint32_t>(pages) - 1;

			before.prev = page > 0;

			if (page == last)
				before.next = false;
			else if (page < last)
				before.next = true;

			return before;
		}
	}
}
