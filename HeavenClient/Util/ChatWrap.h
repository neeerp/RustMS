#pragma once

#include <algorithm>
#include <cstddef>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// How v83 splits a chat log line into rows (FUN_008db070).
	namespace ChatWrap
	{
		// What v83 puts before each row after the first.
		inline const std::string INDENT = "     ";

		inline bool blank(const std::string& text)
		{
			return text.find_first_not_of(" \t\r\n") == std::string::npos;
		}

		// Split the text into rows. `fit` gives how many of a text's first
		// characters fit in the row. The rest goes on the next row, indented
		// if `indent` is set, until only whitespace is left.
		inline std::vector<std::string> wrap(
			std::string text,
			bool indent,
			const std::function<size_t(const std::string&)>& fit
		)
		{
			std::vector<std::string> rows;

			while (!blank(text))
			{
				// A character always goes on the row, so a narrow log can't
				// stop the wrapping.
				size_t count = std::max<size_t>(1, std::min(fit(text), text.size()));
				std::string rest = text.substr(count);

				rows.push_back(text.substr(0, count));

				if (indent && !rest.empty())
					rest = INDENT + rest;

				text = rest;
			}

			return rows;
		}
	}
}
