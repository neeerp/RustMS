#pragma once

#include <cctype>
#include <string>
#include <vector>

namespace ms
{
	// v83's chat filter, which stars out the words of Etc/Curse.img in
	// other players' chat (FUN_007a06f3).
	namespace CurseWords
	{
		// The characters the filter skips over, so "f.o.o" still matches
		// "foo"
		inline bool ignored(char c)
		{
			static const std::string IGNORED = " _-:\t^.,*/;!\\'\"`+";

			return IGNORED.find(c) != std::string::npos;
		}

		// Where the word first appears in the text, ignoring case. As in
		// v83, no match starts at the text's last character.
		inline size_t find(const std::string& text, const std::string& word)
		{
			if (word.empty())
				return std::string::npos;

			auto upper = [](char c) { return std::toupper(static_cast<unsigned char>(c)); };

			for (size_t start = 0; start + 1 < text.size() && start + word.size() <= text.size(); start++)
			{
				size_t i = 0;

				while (i < word.size() && upper(text[start + i]) == upper(word[i]))
					i++;

				if (i == word.size())
					return start;
			}

			return std::string::npos;
		}

		// Replace each word, in order, with as many stars. Words are found
		// with the skipped characters taken out, and those characters stay
		// where they were. A message with control characters other than
		// line breaks, or of 1023 bytes or more, is left as it is.
		inline std::string filter(const std::string& message, const std::vector<std::string>& words)
		{
			if (message.size() >= 0x3ff)
				return message;

			for (char c : message)
				if (static_cast<unsigned char>(c) < 0x20 && c != '\r' && c != '\n')
					return message;

			// Line breaks are skipped like the ignored characters.
			auto kept = [](char c) { return static_cast<unsigned char>(c) >= 0x20 && !ignored(c); };
			std::string squeezed;

			for (char c : message)
				if (kept(c))
					squeezed += c;

			for (const auto& word : words)
			{
				// Its own stars would match a word with one, forever.
				if (word.find('*') != std::string::npos)
					continue;

				for (size_t at = find(squeezed, word); at != std::string::npos; at = find(squeezed, word))
					squeezed.replace(at, word.size(), word.size(), '*');
			}

			std::string filtered = message;
			size_t next = 0;

			for (char& c : filtered)
				if (next < squeezed.size() && kept(c))
					c = squeezed[next++];

			return filtered;
		}

		// v83 shows nearby and group chat with every character outside
		// space to tilde as a space.
		inline std::string printable(std::string message)
		{
			for (char& c : message)
				if (c < ' ' || c > '~')
					c = ' ';

			return message;
		}
	}
}
