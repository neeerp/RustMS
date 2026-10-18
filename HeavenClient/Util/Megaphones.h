#pragma once

#include <cstdint>
#include <string>

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
	}
}
