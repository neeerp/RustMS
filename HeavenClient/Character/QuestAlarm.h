#pragma once

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <map>
#include <sstream>
#include <string>
#include <vector>

// The Quest Helper's list as v83 keeps it, in CConfig's per-character section
// (load 0x49e536, save 0x49e631).
namespace ms::QuestAlarm
{
	// At most five quests are listed (0x49ed6a, 0x885aa8).
	constexpr size_t MAX = 5;
	// A quest added automatically leaves after ten minutes without progress (0x887b5c).
	constexpr int64_t IDLE_MS = 600000;

	// One character's QuestAlarm_cnt, QuestAlarm_%d and QuestAlarm_autoregister
	// (0x49e536), AUTO on by default (0x49d94a).
	struct Saved
	{
		std::vector<int16_t> quests;
		bool auto_register = true;

		bool operator==(const Saved& other) const
		{
			return quests == other.quests && auto_register == other.auto_register;
		}
	};

	// The character's key, as v83's g<world>_<id> registry key.
	inline std::string key(uint8_t world, int32_t character)
	{
		return std::to_string(world) + "_" + std::to_string(character);
	}

	// Every character's lists in one setting: "key:auto:id,id;key:auto:".
	inline std::map<std::string, Saved> parse(const std::string& all)
	{
		std::map<std::string, Saved> result;
		std::stringstream entries(all);
		std::string entry;

		while (std::getline(entries, entry, ';'))
		{
			size_t first = entry.find(':');
			size_t second = first == std::string::npos ? first : entry.find(':', first + 1);

			if (first == 0 || second == std::string::npos || second != first + 2)
				continue;

			Saved saved;
			saved.auto_register = entry[first + 1] != '0';
			std::stringstream ids(entry.substr(second + 1));
			std::string id;

			while (std::getline(ids, id, ',') && saved.quests.size() < MAX)
			{
				if (id.empty() || id.size() > 5 || !std::all_of(id.begin(), id.end(), [](unsigned char c) { return std::isdigit(c) != 0; }))
					continue;

				int32_t value = std::stoi(id);

				if (value > 0 && value <= INT16_MAX
					&& std::find(saved.quests.begin(), saved.quests.end(), value) == saved.quests.end())
					saved.quests.push_back(static_cast<int16_t>(value));
			}

			result[entry.substr(0, first)] = saved;
		}

		return result;
	}

	inline std::string format(const std::map<std::string, Saved>& lists)
	{
		std::string result;

		for (const auto& [name, saved] : lists)
		{
			if (!result.empty())
				result += ';';

			result += name + ':' + (saved.auto_register ? '1' : '0') + ':';

			for (size_t i = 0; i < saved.quests.size(); i++)
				result += (i ? "," : "") + std::to_string(saved.quests[i]);
		}

		return result;
	}

	// The character's list, or an empty one with AUTO on if none was saved.
	inline Saved find(const std::string& all, const std::string& name)
	{
		auto lists = parse(all);
		auto found = lists.find(name);

		return found == lists.end() ? Saved() : found->second;
	}

	inline std::string store(const std::string& all, const std::string& name, const Saved& saved)
	{
		auto lists = parse(all);
		lists[name] = saved;

		return format(lists);
	}

	// Where a quest added by hand goes: above the first one added
	// automatically (0x885d4a).
	inline size_t manual_slot(const std::vector<int16_t>& listed, const std::map<int16_t, int64_t>& automatic)
	{
		for (size_t i = 0; i < listed.size(); i++)
			if (automatic.count(listed[i]))
				return i;

		return listed.size();
	}
}
