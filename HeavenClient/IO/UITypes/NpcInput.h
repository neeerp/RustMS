#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// The rules of an NPC's dialogue answers and of its text prompts (v83
	// CUtilDlgEx; the number prompt uses enter_number).
	namespace npc_input
	{
		// What a dialogue closed without an answer (End Chat or Esc) sends:
		// -1 for say, yes/no and accept/decline, 0 for the rest (7467ab to
		// 746fad).
		inline int8_t close_answer(int8_t msgtype)
		{
			return msgtype == 0 || msgtype == 1 || msgtype == 12 ? -1 : 0;
		}

		// A text prompt's answer: what was typed, less spaces, tabs and line
		// breaks at either end (474414, 4744c9).
		inline std::string trim(const std::string &text)
		{
			const char *blank = " \t\r\n";
			size_t first = text.find_first_not_of(blank);

			if (first == std::string::npos)
				return "";

			return text.substr(first, text.find_last_not_of(blank) - first + 1);
		}

		// The notice for a trimmed answer shorter than `min_length` bytes
		// (StringPool 0x335), or nothing.
		inline std::string text_problem(const std::string &trimmed, int32_t min_length)
		{
			if (static_cast<int64_t>(trimmed.size()) >= min_length)
				return "";

			return "Please enter at least " + std::to_string(min_length) + " characters.";
		}
	}
}
