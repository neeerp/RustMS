#pragma once

#include <array>
#include <cstdint>
#include <string>

namespace ms
{
	// The rules of an NPC's quiz (dialogue type 5, v83 CQuizDlg) and speed quiz (type 6).
	namespace quiz
	{
		// The whole seconds in `left_ms`, 0 once it has run out (7eb16e, 7ec7cb). The time is up as soon as
		// less than a second is left.
		inline int32_t seconds_left(int64_t left_ms)
		{
			return left_ms > 0 ? static_cast<int32_t>(left_ms / 1000) : 0;
		}

		// The countdown's digits, as minutes and seconds (7e9dc6 to 7ea62d).
		inline std::array<int32_t, 4> countdown(int32_t seconds)
		{
			int32_t minutes = seconds / 60, rest = seconds % 60;

			return { minutes / 10, minutes % 10, rest / 10, rest % 10 };
		}

		// The notice for a quiz answer of the wrong length, or nothing (7eae53). v83 counts the answer's bytes
		// against twice the letters asked for, two bytes a Korean letter, and its notices give the doubled
		// numbers.
		inline std::string length_problem(const std::string &answer, int32_t min_letters, int32_t max_letters)
		{
			int64_t length = static_cast<int64_t>(answer.size());
			int64_t least = 2 * static_cast<int64_t>(min_letters);
			int64_t most = 2 * static_cast<int64_t>(max_letters);

			if (length < least)
				return "You must enter atleast " + std::to_string(least) + " letters. (Korean)";

			if (length > most)
				return "You must enter less than " + std::to_string(most) + " letters. (Korean)";

			return "";
		}

		// How loud, out of 100, the speed quiz ticks with `seconds` of `total` left (7ec847). It ticks each
		// second of the last two minutes.
		inline int32_t tick_volume(int32_t seconds, int32_t total)
		{
			if (total <= 0)
				return 100;

			return static_cast<int32_t>((1.0 - static_cast<double>(seconds) / total) * 100);
		}
	}
}
