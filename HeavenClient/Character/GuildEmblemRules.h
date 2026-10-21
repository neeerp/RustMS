#pragma once

#include "Guild.h"

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The rules of Lea's emblem maker (v83 FUN_005950c3, FUN_005952a4 and FUN_0059540e, report 248): what it
	// starts from, how its arrows step through the ids and colours, and the category combo.
	namespace emblem
	{
		constexpr uint8_t COLORS = 16;

		// The combo's categories, in order: Animal, Plant, Pattern, Character and ETC.
		constexpr uint16_t CATEGORIES = 5;

		// The last id of each category of GuildMark.img, by its thousands: 1 the backgrounds, 2 to 5 and 9 the
		// marks.
		using LastIds = std::array<uint16_t, 10>;

		// The last id of a category, from its children's names (FUN_005954cb takes the last child's; all are
		// eight-digit ids, in order). 0 for none.
		inline uint16_t last_id(const std::vector<std::string>& names)
		{
			uint16_t last = 0;

			for (const auto& name : names)
			{
				unsigned long id = 0;

				for (char c : name)
				{
					if (c < '0' || c > '9' || id > UINT16_MAX)
					{
						id = 0;
						break;
					}

					id = id * 10 + static_cast<unsigned long>(c - '0');
				}

				if (id <= UINT16_MAX && id > last)
					last = static_cast<uint16_t>(id);
			}

			return last;
		}

		// The guild's emblem, if it has a mark; otherwise the first of everything, background too.
		inline GuildEmblem initial(const GuildEmblem& current)
		{
			if (current.mark != 0)
				return current;

			return GuildEmblem{ 1000, 1, 2000, 1 };
		}

		// The combo entry of a mark's category.
		inline uint16_t category_index(uint16_t mark)
		{
			uint16_t category = mark / 1000;

			if (category == 9)
				return CATEGORIES - 1;

			if (category >= 2 && category <= 5)
				return category - 2;

			return 0;
		}

		// The first mark of a combo entry's category.
		inline uint16_t category_mark(uint16_t index)
		{
			if (index >= CATEGORIES - 1)
				return 9000;

			return static_cast<uint16_t>((index + 2) * 1000);
		}

		// The background after an arrow, left or right: from 1000 to the last, wrapping.
		inline uint16_t step_background(uint16_t background, bool right, const LastIds& last)
		{
			if (right)
				return background == last[1] ? 1000 : background + 1;

			return background % 1000 == 0 ? last[1] : background - 1;
		}

		// The mark after an arrow, wrapping within its category.
		inline uint16_t step_mark(uint16_t mark, bool right, const LastIds& last)
		{
			uint16_t category = mark / 1000;

			if (category >= last.size())
				return mark;

			if (right)
				return mark == last[category] ? category * 1000 : mark + 1;

			return mark % 1000 == 0 ? last[category] : mark - 1;
		}

		// A colour after an arrow: 1 to 16, wrapping.
		inline uint8_t step_color(uint8_t color, bool right)
		{
			if (right)
				return color == COLORS ? 1 : color + 1;

			return color == 1 ? COLORS : color - 1;
		}
	}
}
