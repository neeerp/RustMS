#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// How v83 shows the lines megaphones send (BROADCAST_MSG types 2, 3 and
	// 8 to 10, FUN_00a22d3a).
	namespace Megaphones
	{
		// What separates the speaker's name from the text.
		inline const std::string SEPARATOR = " : ";

		// Megaphones aren't heard on event maps, but by GMs.
		inline bool heard_on(int32_t mapid)
		{
			return (mapid / 1'000'000) % 100 != 9;
		}

		// The line with the shown item's name before the text, as
		// "Name : [Item]text".
		inline std::string with_item(const std::string& line, const std::string& item)
		{
			size_t split = line.find(SEPARATOR);

			if (split == std::string::npos)
				return "[" + item + "]" + line;

			split += SEPARATOR.size();

			return line.substr(0, split) + "[" + item + "]" + line.substr(split);
		}

		// A triple megaphone's later line, under the first line's name.
		inline std::string under_name(const std::string& first, const std::string& line)
		{
			size_t name = first.find(SEPARATOR);
			size_t text = line.find(SEPARATOR);

			if (name == std::string::npos || text == std::string::npos)
				return line;

			return first.substr(0, name) + line.substr(text);
		}

		// The longest line a megaphone sends (StringPool 0x111), and the
		// longest it drops without a word.
		constexpr size_t MAX_LINE = 60;
		constexpr size_t MAX_TYPED = 80;
		// The plain and super megaphones are for characters over this level
		// (StringPool 0xea1).
		constexpr uint16_t MIN_LEVEL = 10;

		// The megaphones the Cash tab uses, by what they ask for
		// (FUN_0048645b's 507 kinds: 1 and 8 plain, 2 super, 5 from 6 and 6
		// item, 7 triple; 5 below 6 is Maple TV).
		enum class Kind : uint8_t
		{
			NONE, PLAIN, SUPER, ITEM, TRIPLE
		};

		inline Kind kind(int32_t item_id)
		{
			if (item_id / 10'000 != 507)
				return Kind::NONE;

			switch (item_id % 10'000 / 1'000)
			{
				case 1:
				case 8:
					return Kind::PLAIN;
				case 2:
					return Kind::SUPER;
				case 5:
					return item_id % 10 >= 6 ? Kind::ITEM : Kind::NONE;
				case 6:
					return Kind::ITEM;
				case 7:
					return Kind::TRIPLE;
				default:
					return Kind::NONE;
			}
		}

		// Why a line isn't sent. QUIET says nothing.
		enum class Refusal : uint8_t
		{
			NONE, QUIET, CURSING, LEVEL, LONG
		};

		// Checks a plain or super megaphone's line in v83's order, once its
		// prompt has closed (FUN_00a0a63f types 0xc, 0xd and 0xf). `censor`
		// stars out curse words.
		template <typename Censor>
		Refusal check_line(const std::string& text, uint16_t level, Censor censor)
		{
			if (text.empty())
				return Refusal::QUIET;

			if (censor(text) != text)
				return Refusal::CURSING;

			if (level <= MIN_LEVEL)
				return Refusal::LEVEL;

			if (text.size() > MAX_TYPED)
				return Refusal::QUIET;

			if (text.size() > MAX_LINE)
				return Refusal::LONG;

			return Refusal::NONE;
		}

		// A triple megaphone checks only that something is written, and how
		// long each line is (type 0x3c).
		inline Refusal check_lines(const std::vector<std::string>& lines)
		{
			bool written = false;

			for (const std::string& line : lines)
			{
				if (line.size() > MAX_LINE)
					return Refusal::LONG;

				written = written || !line.empty();
			}

			return written ? Refusal::NONE : Refusal::QUIET;
		}

		// The lines a triple megaphone sends: up to the last one written,
		// with any blank ones before it.
		inline std::vector<std::string> lines_sent(const std::vector<std::string>& lines)
		{
			size_t count = lines.size();

			while (count > 1 && lines[count - 1].empty())
				count--;

			return std::vector<std::string>(lines.begin(), lines.begin() + count);
		}
	}
}
