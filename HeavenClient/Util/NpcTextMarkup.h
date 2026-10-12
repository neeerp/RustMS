#pragma once

#include <cstdint>
#include <functional>
#include <string>

namespace ms
{
	// Expands the name and image tags of NPC/quest text. Colour tags (#b, #k,
	// ...) and #L choices are left for the text renderer and menu parser.
	namespace NpcTextMarkup
	{
		struct Lookups
		{
			std::function<std::string()> player_name;
			std::function<std::string(int32_t)> npc_name;
			std::function<std::string(int32_t)> item_name;
			std::function<std::string(int32_t)> mob_name;
			std::function<std::string(int32_t)> map_name;
		};

		inline bool is_digits(const std::string& s)
		{
			if (s.empty())
				return false;

			for (char c : s)
				if (c < '0' || c > '9')
					return false;

			return true;
		}

		inline std::string expand(const std::string& text, const Lookups& lookups)
		{
			std::string out;
			size_t i = 0;

			while (i < text.size())
			{
				if (text[i] != '#' || i + 1 >= text.size())
				{
					out += text[i++];
					continue;
				}

				char tag = text[i + 1];
				size_t close = text.find('#', i + 2);
				std::string arg = close == std::string::npos ? std::string() : text.substr(i + 2, close - i - 2);

				auto id = [&]() { return static_cast<int32_t>(std::stol(arg)); };
				bool numeric = is_digits(arg);

				switch (tag)
				{
					case 'h':
						// #h0# or "#h #": the player's name.
						if (close != std::string::npos && (numeric || arg == " " || arg.empty()))
						{
							out += lookups.player_name();
							i = close + 1;
							continue;
						}
						break;
					case 'p':
						if (numeric) { out += lookups.npc_name(id()); i = close + 1; continue; }
						break;
					case 't':
					case 'z':
						if (numeric) { out += lookups.item_name(id()); i = close + 1; continue; }
						break;
					case 'o':
						if (numeric) { out += lookups.mob_name(id()); i = close + 1; continue; }
						break;
					case 'm':
						if (numeric) { out += lookups.map_name(id()); i = close + 1; continue; }
						break;
					case 'v':
					case 'i':
					case 's':
						// Item and skill icons: not drawn inline yet.
						if (numeric) { i = close + 1; continue; }
						break;
					case 'f':
					case 'F':
						// UI images by path: not drawn inline yet.
						if (close != std::string::npos && arg.find('/') != std::string::npos)
						{
							i = close + 1;
							continue;
						}
						break;
					default:
						break;
				}

				out += text[i++];
			}

			return out;
		}
	}
}
