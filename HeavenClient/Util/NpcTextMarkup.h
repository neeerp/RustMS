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
		// Stands in for an inline image in expanded text; the text layout
		// gives it the image's size.
		constexpr char IMAGE = '\x01';

		// An image tag: #v/#i (item icon), #s (skill icon) or #f/#F (a
		// path such as UI/UIWindow.img/QuestIcon/4/0).
		struct InlineImage
		{
			char tag;
			std::string arg;

			bool operator==(const InlineImage& other) const
			{
				return tag == other.tag && arg == other.arg;
			}
		};

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

		// Without `images`, image tags are dropped; with it, each becomes an
		// IMAGE placeholder and is appended to `images` in order.
		inline std::string expand(const std::string& text, const Lookups& lookups,
								  std::vector<InlineImage>* images = nullptr)
		{
			auto image = [&](char tag, const std::string& arg)
			{
				if (images)
				{
					images->push_back({ tag, arg });
					return std::string(1, IMAGE);
				}

				return std::string();
			};

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

				// v83 writes ids as "#v2000002:#" as well as "#v2000002#".
				std::string number = !arg.empty() && arg.back() == ':' ? arg.substr(0, arg.size() - 1) : arg;
				auto id = [&]() { return static_cast<int32_t>(std::stol(number)); };
				bool numeric = is_digits(number);

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
						// Item and skill icons.
						if (numeric) { out += image(tag, number); i = close + 1; continue; }
						break;
					case 'f':
					case 'F':
						// Images by path.
						if (close != std::string::npos && arg.find('/') != std::string::npos)
						{
							out += image(tag, arg);
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

		// The colour code (#k, #r, #g, #b or #d) in effect at `pos`, or
		// nothing, as for "#d" before each line of an NPC's quest menu.
		inline std::string colour_at(const std::string& text, size_t pos)
		{
			std::string colour;

			for (size_t i = 0; i + 1 < std::min(pos, text.size()); i++)
			{
				if (text[i] != '#')
					continue;

				char tag = text[i + 1];

				if (tag == 'L')
				{
					// Skip a choice's id: "#L1#bananas" is not blue.
					i = text.find('#', i + 2);

					if (i == std::string::npos)
						break;

					continue;
				}

				if (std::string("krgbd").find(tag) != std::string::npos)
					colour = text.substr(i, 2);

				i++;
			}

			return colour;
		}

		// Where a choice starts and where its label ends, in text marked by
		// mark_choices.
		constexpr char CHOICE = '\x02';
		constexpr char CHOICE_END = '\x03';

		// Marks the choices of a menu for laying them out in the text: each
		// #L<id># becomes CHOICE, and CHOICE_END follows its label as
		// choices() trims it. A closing #l is dropped. Appends the ids to
		// `ids` in order.
		inline std::string mark_choices(const std::string& text, std::vector<int32_t>& ids)
		{
			std::string out;
			size_t done = 0;
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
				size_t length = label.size();

				while (size_t n = trailing_filler(label.substr(0, length)))
					length -= n;

				ids.push_back(static_cast<int32_t>(std::stol(text.substr(start + 2, id_end - start - 2))));
				out += text.substr(done, start - done);
				out += CHOICE;
				out += label.substr(0, length);
				out += CHOICE_END;
				out += label.substr(length);

				done = end == std::string::npos ? text.size() : end == close ? close + 2 : next;
				start = next;
			}

			return out + text.substr(std::min(done, text.size()));
		}
	}
}
