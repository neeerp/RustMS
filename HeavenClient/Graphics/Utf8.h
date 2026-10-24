//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include <array>
#include <cstddef>
#include <string>

namespace ms
{
	// Game text is UTF-8 in the NX files. v83 drew the same strings from
	// code page 1252, so the fonts carry that page's characters.
	namespace Utf8
	{
		constexpr char32_t REPLACEMENT = U'?';

		inline bool is_continuation(char c)
		{
			return (static_cast<unsigned char>(c) & 0xC0) == 0x80;
		}

		// The character starting at `pos` and its length in bytes. A byte in the
		// middle of a character has length 0; a malformed sequence is one byte of
		// REPLACEMENT.
		inline size_t decode(const std::string &text, size_t pos, char32_t &codepoint)
		{
			auto byte = static_cast<unsigned char>(text[pos]);

			if (byte < 0x80)
			{
				codepoint = byte;
				return 1;
			}

			if (is_continuation(text[pos]))
			{
				codepoint = 0;
				return 0;
			}

			size_t length;
			char32_t value;

			if ((byte & 0xE0) == 0xC0)
			{
				length = 2;
				value = byte & 0x1F;
			} else if ((byte & 0xF0) == 0xE0)
			{
				length = 3;
				value = byte & 0x0F;
			} else if ((byte & 0xF8) == 0xF0)
			{
				length = 4;
				value = byte & 0x07;
			} else
			{
				codepoint = REPLACEMENT;
				return 1;
			}

			if (pos + length > text.size())
			{
				codepoint = REPLACEMENT;
				return 1;
			}

			for (size_t i = 1; i < length; i++)
			{
				if (!is_continuation(text[pos + i]))
				{
					codepoint = REPLACEMENT;
					return 1;
				}

				value = (value << 6) | (static_cast<unsigned char>(text[pos + i]) & 0x3F);
			}

			codepoint = value;
			return length;
		}

		// The bytes from `pos` up to the next character's start (at most `last`).
		inline size_t span(const std::string &text, size_t pos, size_t last)
		{
			size_t end = pos + 1;

			while (end < last && is_continuation(text[end]))
				end++;

			return end;
		}

		// Code page 1252's characters outside ASCII and Latin-1.
		constexpr std::array<char32_t, 27> CP1252_EXTRAS = {
			0x20AC, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030,
			0x0160, 0x2039, 0x0152, 0x017D, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022,
			0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x017E, 0x0178
		};

		// Symbols the game's English text uses beyond code page 1252, as the
		// title quests' "● Status" and the arrows in Etc's guides. v83 drew
		// them with Windows' font fallback; here the face's own glyph, or '?'.
		constexpr std::array<char32_t, 15> SYMBOLS = {
			0x25CF, 0x25A1, 0x25B3, 0x2665, 0x2661, 0x2605, 0x266A, 0x266C,
			0x2190, 0x2191, 0x2192, 0x2193, 0x203B, 0x2161, 0x2163
		};
	}
}
