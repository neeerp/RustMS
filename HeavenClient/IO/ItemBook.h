#pragma once

#include <cstdint>
#include <cstdlib>
#include <string>

namespace ms
{
	// The Book window of an Etc 416xxxx item (v83's, ctor 0x461a9a, report p332): its pages two at a time, the even
	// one on the left, turned by Prev and Next.
	namespace item_book
	{
		// Any Etc item of 416xxxx opens it on a double click (0x4eff77), whatever its data.
		inline bool opens(int32_t item_id)
		{
			return item_id / 10000 == 416;
		}

		// The spread a page is on: its even page (0x4626fe).
		inline int32_t spread(int32_t page)
		{
			return page & ~1;
		}

		// Where Prev and Next go from the spread showing `page`, or -1 where they don't (0x4626fe, 0x462708).
		// `last` is the last page's index.
		inline int32_t previous(int32_t page)
		{
			int32_t to = spread(page) - 2;

			return to < 0 ? -1 : to;
		}

		inline int32_t next(int32_t page, int32_t last)
		{
			int32_t to = spread(page) + 2;

			return to > last ? -1 : to;
		}

		// Whether each is enabled (0x46320d).
		inline bool has_previous(int32_t page)
		{
			return page >= 2;
		}

		inline bool has_next(int32_t page, int32_t last)
		{
			return spread(page) <= last - 2;
		}

		// Where a #L<n># link goes: the spread with page n (0x462770), or -1 past the last page.
		inline int32_t link_target(int32_t link, int32_t last)
		{
			return link < 0 || link > last ? -1 : spread(link);
		}

		// A page's number under it (0x462aec): "1/36" for the first of 36.
		inline std::string page_label(int32_t page, int32_t last)
		{
			return std::to_string(page + 1) + "/" + std::to_string(last + 1);
		}

		enum class Align
		{
			LEFT,
			CENTER,
			RIGHT
		};

		// An entry's `align` (0x463110): 1 centres each line, 2 puts it on the right, anything else on the left.
		// The data writes it as an int, or as a string such as "1" or "1.000000".
		inline Align align(int64_t value)
		{
			return value == 1 ? Align::CENTER : value == 2 ? Align::RIGHT : Align::LEFT;
		}

		inline Align align(const std::string &value)
		{
			return align(static_cast<int64_t>(std::atoi(value.c_str())));
		}
	}
}
