#pragma once

#include "../Data/CurseData.h"

#include <algorithm>
#include <array>
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

		// A Gachapon win told to the world (BROADCAST_MSG type 11, 0xa233cb):
		// the line "Name : got a(n)" around the prize and the machine's town,
		// as StringPool 0x137d "%s %s  :  %s  from the %s Gachapon! Congrats!".
		inline std::string gachapon(const std::string& line, const std::string& item, const std::string& town)
		{
			size_t split = line.find(SEPARATOR);
			std::string name = line.substr(0, split);
			std::string won = split == std::string::npos ? "" : line.substr(split + SEPARATOR.size());

			return name + " " + won + "  :  " + item + "  from the " + town + " Gachapon! Congrats!";
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
		// item, 7 triple; 5075000 to 5075005 are Maple TV), and the avatar
		// megaphones (539).
		enum class Kind : uint8_t
		{
			NONE, PLAIN, SUPER, ITEM, TRIPLE, AVATAR, TV
		};

		inline Kind kind(int32_t item_id)
		{
			if (item_id / 10'000 == 539)
				return Kind::AVATAR;

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
					if (item_id % 10 >= 6)
						return Kind::ITEM;

					return item_id / 10 == 507'500 ? Kind::TV : Kind::NONE;
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
			NONE, QUIET, CURSING, LEVEL, LONG, EMPTY
		};

		// Checks a plain or super megaphone's line in v83's order, once its
		// prompt has closed (FUN_00a0a63f types 0xc, 0xd and 0xf). `text`
		// becomes what is sent, its curse words starred out by `censor`.
		template <typename Censor>
		Refusal check_line(std::string& text, uint16_t level, Censor censor)
		{
			if (text.empty())
				return Refusal::QUIET;

			if (!CurseData::valid(text))
				return Refusal::CURSING;

			text = censor(text);

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

		// A triple megaphone's sample line as v83 puts it in a field: through
		// the ANSI code page, where anything past ASCII here becomes '?'
		// (FUN_007e1052). Latin-1 letters would survive on a Western page, but
		// the samples have none.
		inline std::string sample_line(const std::string& utf8)
		{
			std::string line;

			for (size_t i = 0; i < utf8.size();)
			{
				auto lead = static_cast<unsigned char>(utf8[i]);

				if (lead < 0x80)
				{
					line += utf8[i];
					i++;
				} else
				{
					line += '?';
					i += lead >= 0xf0 ? 4 : lead >= 0xe0 ? 3 : lead >= 0xc0 ? 2 : 1;
				}
			}

			return line;
		}

		// The avatar megaphone's box (FUN_007f1196): one wrapped field of
		// four lines and 97 characters.
		constexpr size_t AVATAR_LINES = 4;
		constexpr size_t AVATAR_TYPED = 97;

		using AvatarLines = std::array<std::string, AVATAR_LINES>;

		// The first `N` rows of a wrapped field's text, as the avatar
		// megaphone (FUN_007f1525) and Maple TV (0x7f0dc9) send them,
		// `starts` being where each row begins. A row past the last is left
		// out. The space a row wraps at stays on the row before, so the lines
		// run together as typed.
		template <size_t N>
		std::array<std::string, N> wrapped_lines(const std::string& text, const std::vector<size_t>& starts)
		{
			std::array<std::string, N> lines;
			std::vector<size_t> cuts;

			for (size_t start : starts)
			{
				if (start > 0 && start < text.size() && text[start] == ' ')
					start++;

				cuts.push_back(std::min(start, text.size()));
			}

			for (size_t i = 0; i < N && i < cuts.size(); i++)
			{
				size_t end = i + 1 < cuts.size() ? cuts[i + 1] : text.size();

				if (end > cuts[i])
					lines[i] = text.substr(cuts[i], end - cuts[i]);
			}

			return lines;
		}

		inline AvatarLines avatar_lines(const std::string& text, const std::vector<size_t>& starts)
		{
			return wrapped_lines<AVATAR_LINES>(text, starts);
		}

		// Whether the filter takes rows cut from one text (`joined`), and if
		// it does their curse words starred out by `censor`. The text is
		// censored as one, so a word cut by a wrap is caught, then cut back
		// into the rows; stars keep the length.
		template <size_t N, typename Censor>
		bool censor_rows(std::array<std::string, N>& lines, const std::string& joined, Censor censor)
		{
			if (!CurseData::valid(joined))
				return false;

			std::string censored = censor(joined);
			size_t start = 0;

			for (std::string& line : lines)
			{
				line = censored.substr(start, line.size());
				start += line.size();
			}

			return true;
		}

		// Checks an avatar megaphone's lines once its box has closed: v83
		// asks for some text, but has no level or length rule of its own
		// (FUN_00a0a63f's 539 branch). Each line's curse words are starred
		// out by `censor`.
		template <typename Censor>
		Refusal check_avatar_lines(AvatarLines& lines, Censor censor)
		{
			std::string joined;

			for (const std::string& line : lines)
				joined += line;

			if (joined.empty())
				return Refusal::EMPTY;

			return censor_rows(lines, joined, censor) ? Refusal::NONE : Refusal::CURSING;
		}

		// Where an avatar megaphone's banner (FUN_0045a825) puts the name:
		// the short plate (`name/0`) under a name narrower than 70, at x 48,
		// and the long one (`name/1`) at x 3.
		constexpr int16_t AVATAR_SHORT_NAME = 70;

		inline bool avatar_long_name(int16_t name_width)
		{
			return name_width >= AVATAR_SHORT_NAME;
		}
	}
}
