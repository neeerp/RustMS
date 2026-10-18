#pragma once

#include <algorithm>
#include <cctype>
#include <string>

namespace ms
{
	// How v83 reads the chat bar's slash commands.
	namespace ChatCommand
	{
		// Whether a and b match without regard to case.
		inline bool same_text(const std::string& a, const std::string& b)
		{
			return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(),
				[](unsigned char x, unsigned char y) { return std::tolower(x) == std::tolower(y); });
		}

		// Whether input is command followed by a space and something more,
		// which becomes rest (FUN_004a8e3d).
		inline bool with_argument(const std::string& input, const std::string& command, std::string& rest)
		{
			std::string prefix = command + " ";

			if (input.size() <= prefix.size() || !same_text(input.substr(0, prefix.size()), prefix))
				return false;

			rest = input.substr(prefix.size());
			return true;
		}
	}
}
