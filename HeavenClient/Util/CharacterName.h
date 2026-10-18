#pragma once

#include <algorithm>
#include <string>

namespace ms
{
	// Whether a typed name could be a character's, as v83 checks it before
	// asking the server (FUN_0079fc74), but for the forbidden words some
	// callers add: 4 to 12 letters or digits, with fewer than four of 'I'
	// and 'l' between them.
	inline bool character_name_valid(const std::string& name)
	{
		if (name.size() < 4 || name.size() > 12)
			return false;

		bool letters = std::all_of(name.begin(), name.end(), [](char c)
		{
			return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9');
		});

		return letters && std::count_if(name.begin(), name.end(), [](char c) { return c == 'I' || c == 'l'; }) < 4;
	}
}
