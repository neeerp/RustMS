#pragma once

#include <cctype>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The word lists of Etc/Swindle.img, which v83 checks chat against to
	// print a scam warning (FUN_007a09ca).
	namespace SwindleWords
	{
		// One numbered entry: the words that trigger it and the warnings to
		// pick from.
		struct Group
		{
			std::vector<std::string> words;
			std::vector<std::string> warnings;
		};

		// Drop control characters and the punctuation v83 ignores. Words
		// aren't cleaned this way, so a word with a space never matches.
		inline std::string normalize(const std::string& message)
		{
			static const std::string IGNORED = " _-:\t^.,*/;!\\'\"`+";

			std::string normalized;

			for (char c : message)
				if (static_cast<unsigned char>(c) >= 0x20 && IGNORED.find(c) == std::string::npos)
					normalized += c;

			return normalized;
		}

		// Whether the word appears in the normalized message, ignoring case.
		// As in v83, no match starts at the message's last character.
		inline bool contains(const std::string& normalized, const std::string& word)
		{
			if (word.empty())
				return false;

			auto upper = [](char c) { return std::toupper(static_cast<unsigned char>(c)); };

			for (size_t start = 0; start + 1 < normalized.size(); start++)
			{
				if (start + word.size() > normalized.size())
					break;

				size_t i = 0;

				while (i < word.size() && upper(normalized[start + i]) == upper(word[i]))
					i++;

				if (i == word.size())
					return true;
			}

			return false;
		}

		// The group whose word the message has, checking the last group
		// first and each group's words in order.
		inline std::optional<size_t> match(const std::vector<Group>& groups, const std::string& message)
		{
			std::string normalized = normalize(message);

			for (size_t g = groups.size(); g-- > 0;)
				for (const auto& word : groups[g].words)
					if (contains(normalized, word))
						return g;

			return std::nullopt;
		}
	}
}
