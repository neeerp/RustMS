#pragma once

#include <cstdint>
#include <functional>
#include <string>
#include <utility>
#include <vector>

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
	
		// The length of a line break (escaped or not), colour code, or space
		// at the end of `text`, or zero.
		inline size_t trailing_filler(const std::string& text)
		{
			for (const std::string tail : { "\\r", "\\n", "\r", "\n", "#b", "#k", "#r", "#g", "#d", "#e", "#n", " " })
				if (text.size() >= tail.size() && text.compare(text.size() - tail.size(), tail.size(), tail) == 0)
					return tail.size();

			return 0;
		}

		// The #L<id># choices of a menu, as (id, label). A label ends at its
		// #l, at the next #L, or at the end of the text: the data sometimes
		// leaves out the #l (Robin's last quiz question). Trailing line
		// breaks, escaped or not, and colour codes are trimmed.
		inline std::vector<std::pair<int32_t, std::string>> choices(const std::string& text)
		{
			std::vector<std::pair<int32_t, std::string>> result;
			size_t start = text.find("#L");

			while (start != std::string::npos)
			{
				size_t id_end = text.find('#', start + 2);

				if (id_end == std::string::npos || !is_digits(text.substr(start + 2, id_end - start - 2)))
					break;

				size_t next = text.find("#L", id_end + 1);
				size_t close = text.find("#l", id_end + 1);
				size_t end = std::min(close, next);
				std::string label = text.substr(id_end + 1, end == std::string::npos ? std::string::npos : end - id_end - 1);

				while (size_t n = trailing_filler(label))
					label.erase(label.size() - n);

				result.emplace_back(static_cast<int32_t>(std::stol(text.substr(start + 2, id_end - start - 2))), label);
				start = next;
			}

			return result;
		}
	}
}
