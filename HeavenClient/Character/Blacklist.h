#pragma once

#include "../Util/CharacterName.h"

#include <algorithm>
#include <cctype>
#include <string>
#include <vector>

// The User List's Blacklist tab (reports b089, b090): names the player's shops turn away. v83 keeps them in
// the client's settings, for every character on the machine, and tells the server at a shop's opening.
namespace ms::Blacklist
{
	// AddBlackList's cap (0x49ec27); the tab shows them all, two to a row.
	constexpr size_t MAX = 26;
	// The Add prompt's (0x919214).
	constexpr size_t MIN_NAME = 4;
	constexpr size_t MAX_NAME = 12;

	// The live sync (0x53792e, 0x5379a6) runs only while the player's own personal shop is open: set by its
	// open (0x6fcb82), cleared when its owner's window goes (0x6fa287). A hired merchant never sets it.
	inline bool live = false;

	inline bool same_name(const std::string& name, const std::string& other)
	{
		return name.size() == other.size() && std::equal(name.begin(), name.end(), other.begin(), [](char a, char b)
		{
			return std::tolower(static_cast<unsigned char>(a)) == std::tolower(static_cast<unsigned char>(b));
		});
	}

	inline bool contains(const std::vector<std::string>& names, const std::string& name)
	{
		return std::any_of(names.begin(), names.end(), [&](const std::string& listed) { return same_name(listed, name); });
	}

	// Letter for letter, as 0x49ed00 looks a sender or an inviter up (report b091).
	inline bool listed(const std::vector<std::string>& names, const std::string& name)
	{
		return std::find(names.begin(), names.end(), name) != names.end();
	}

	// The same against the saved list (UIUserListBlacklist.cpp). Whispers, buddy, guild and alliance chat, and
	// invitations from a listed name are turned away as if the Game Option blocked them.
	bool listed(const std::string& name);

	// The settings' list, split by commas, which no name holds, in the order added. As the load does
	// (0x49e303), names that fail the filter or repeat one letter for letter (0x49ed00) are dropped, and
	// the rest cut at MAX.
	inline std::vector<std::string> parse(const std::string& text)
	{
		std::vector<std::string> names;
		size_t start = 0;

		while (start <= text.size() && names.size() < MAX)
		{
			size_t end = text.find(',', start);

			if (end == std::string::npos)
				end = text.size();

			std::string name = text.substr(start, end - start);

			if (character_name_valid(name) && !listed(names, name))
				names.push_back(name);

			start = end + 1;
		}

		return names;
	}

	inline std::string format(const std::vector<std::string>& names)
	{
		std::string text;

		for (const std::string& name : names)
			text += (text.empty() ? "" : ",") + name;

		return text;
	}

	// The tab's order (0x9188d5): byte by byte, capitals first.
	inline std::vector<std::string> sorted(std::vector<std::string> names)
	{
		std::sort(names.begin(), names.end());

		return names;
	}

	// The Add button's checks once the prompt is answered, in v83's order (0x919214).
	enum class Refusal
	{
		NONE,
		// Fails the name filter (0x79fc74), forbidden words included: SP 0x2d7.
		INVALID,
		// The player's own name, letter for letter: SP 0x2d6.
		SELF,
		// Listed already, in any case: SP 0x2d8.
		LISTED,
		// Never shown: the button is greyed out by then.
		FULL
	};

	inline Refusal check(const std::vector<std::string>& names, const std::string& name, const std::string& self,
						 bool forbidden)
	{
		if (!character_name_valid(name) || forbidden)
			return Refusal::INVALID;

		if (name == self)
			return Refusal::SELF;

		if (contains(names, name))
			return Refusal::LISTED;

		if (names.size() >= MAX)
			return Refusal::FULL;

		return Refusal::NONE;
	}

	inline void remove(std::vector<std::string>& names, const std::string& name)
	{
		names.erase(std::remove(names.begin(), names.end(), name), names.end());
	}
}
