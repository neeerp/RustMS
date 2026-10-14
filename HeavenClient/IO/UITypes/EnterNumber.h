#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// Checks what was typed into a number entry box against 1 to `max`.
	namespace enter_number
	{
		enum class Problem
		{
			NONE,
			NOT_A_NUMBER,
			TOO_LOW,
			TOO_HIGH
		};

		struct Checked
		{
			Problem problem;
			int32_t number;
		};

		inline Checked check(const std::string &text, int32_t max)
		{
			if (text.find_first_not_of("0123456789") != std::string::npos)
				return { Problem::NOT_A_NUMBER, 0 };

			// Leading zeros carry no value; past them, anything longer than `max` is too high without parsing.
			size_t first = text.find_first_not_of('0');

			if (first == std::string::npos)
				return { Problem::TOO_LOW, 0 };

			std::string digits = text.substr(first);

			if (digits.size() > std::to_string(max).size())
				return { Problem::TOO_HIGH, 0 };

			int64_t number = std::stoll(digits);

			if (number > max)
				return { Problem::TOO_HIGH, 0 };

			return { Problem::NONE, static_cast<int32_t>(number) };
		}
	}
}
