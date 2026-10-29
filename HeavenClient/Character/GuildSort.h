#pragma once

#include "Guild.h"

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The order of a section of the guild or alliance tab, picked by a click
	// on its column headings (v83 FUN_00911701 for the guild, FUN_00915456
	// for the alliance).
	namespace guild_sort
	{
		enum Column : int8_t
		{
			NONE,
			NAME,
			JOB,
			LEVEL,
			TITLE
		};

		// The headings' edges, from NAME's left to TITLE's right. A heading
		// is hit from 1 to 17 pixels below the headings' top (0x90fb1a).
		inline constexpr int16_t EDGES[] = { 1, 96, 165, 205, 280 };
		inline constexpr int16_t TOP = 1;
		inline constexpr int16_t BOTTOM = 17;

		// The heading at x, y from the headings' top left, or NONE.
		inline Column column_at(int16_t x, int16_t y)
		{
			if (y < TOP || y >= BOTTOM)
				return NONE;

			for (int8_t column = NAME; column <= TITLE; column++)
				if (x >= EDGES[column - 1] && x < EDGES[column])
					return static_cast<Column>(column);

			return NONE;
		}

		// A section starts in the server's order (FUN_0091bb5c). A heading's
		// first click sorts up by it, the next down, and so on (FUN_0091179d).
		struct Sort
		{
			Column column = NONE;
			bool ascending = true;

			void click(Column clicked)
			{
				if (clicked == column)
				{
					ascending = !ascending;
				}
				else
				{
					column = clicked;
					ascending = true;
				}
			}
		};

		// Names compare as _stricmp does, ignoring the case of A to Z.
		inline int compare_names(const std::string& a, const std::string& b)
		{
			auto lower = [](unsigned char c) { return c >= 'A' && c <= 'Z' ? c + 0x20 : c; };

			for (size_t i = 0; i < a.size() || i < b.size(); i++)
			{
				int x = i < a.size() ? lower(a[i]) : 0;
				int y = i < b.size() ? lower(b[i]) : 0;

				if (x != y)
					return x < y ? -1 : 1;
			}

			return 0;
		}

		// Sorts a section's members (FUN_00911fb6, FUN_00915c84). The name
		// column turns about whole; the others turn their values about but
		// keep equal ones by name, A first. The title column sorts by rank,
		// the alliance's in the alliance tab.
		inline void sort(std::vector<const GuildMember*>& members, Sort order, bool alliance)
		{
			if (order.column == NONE)
				return;

			auto value = [&](const GuildMember* member)
			{
				switch (order.column)
				{
				case JOB:
					return member->job;
				case LEVEL:
					return member->level;
				default:
					return alliance ? member->alliance_rank : member->rank;
				}
			};

			std::stable_sort(members.begin(), members.end(), [&](const GuildMember* a, const GuildMember* b)
			{
				if (order.column == NAME)
				{
					int names = compare_names(a->name, b->name);

					return order.ascending ? names < 0 : names > 0;
				}

				if (value(a) != value(b))
					return order.ascending ? value(a) < value(b) : value(a) > value(b);

				return compare_names(a->name, b->name) < 0;
			});
		}
	}
}
