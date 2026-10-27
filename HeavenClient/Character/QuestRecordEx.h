#pragma once

#include <cstdint>
#include <cstdlib>
#include <functional>
#include <string>

namespace ms
{
	// The records the client keeps beside the quest log ("quest record ex"),
	// each "k=v;k=v", and the Quest Log's #j tokens that read them.
	namespace QuestRecordEx
	{
		// The value under `key` in `raw`, or "": split on ';', then each pair
		// on '=' (0x4e7856).
		inline std::string value(const std::string& raw, const std::string& key)
		{
			size_t start = 0;

			while (start <= raw.size())
			{
				size_t end = raw.find(';', start);

				if (end == std::string::npos)
					end = raw.size();

				std::string pair = raw.substr(start, end - start);
				size_t eq = pair.find('=');

				if (eq != std::string::npos && pair.substr(0, eq) == key)
					return pair.substr(eq + 1);

				start = end + 1;
			}

			return "";
		}

		// What the #j token for `key` shows, `get` reading the quest's record,
		// as the client's quest text formatter (0x99dd54) writes it: counts
		// "%4d" (the seconds "%2d"), the give-ups the tries less the rest, a
		// reward "Acquire" or "Fail", the rank as its image (F if none), and
		// anything else as it stands.
		inline std::string token(const std::string& key, const std::function<std::string(const std::string&)>& get)
		{
			auto number = [&](const std::string& k) { return std::atoi(get(k).c_str()); };
			auto padded = [](int32_t n, size_t width)
			{
				std::string digits = std::to_string(n);
				return std::string(digits.size() < width ? width - digits.size() : 0, ' ') + digits;
			};

			if (key == "rank")
			{
				std::string rank = get(key);
				return "#fUI/Basic.img/PQuestRank/" + (rank.empty() ? std::string("F") : rank) + "#";
			}

			if (key == "try" || key == "vic" || key == "cmp" || key == "lose" || key == "draw" || key == "min")
				return padded(number(key), 4);

			if (key == "sec")
				return padded(number(key), 2);

			if (key == "gvup")
				return std::to_string(number("try") - number("vic") - number("lose") - number("draw"));

			if (key.rfind("have", 0) == 0)
				return number(key) > 0 ? "Acquire" : "Fail";

			return get(key);
		}
	}
}
