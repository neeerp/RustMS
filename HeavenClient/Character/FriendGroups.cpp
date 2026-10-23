#include "FriendGroups.h"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace ms::FriendGroups
{
	const std::string DEFAULT = "Default Group";

	bool before(const std::string& group, const std::string& other)
	{
		if (group == DEFAULT)
			return false;

		return other == DEFAULT || group < other;
	}

	// v83 sorts by name, then stably by group, both by insertion (FUN_00921a6e, FUN_00921b4c).
	std::vector<Group> arrange(const std::vector<Buddy>& buddies)
	{
		std::vector<const Buddy*> listed;

		for (const Buddy& buddy : buddies)
			if (buddy.id != 0 && !buddy.pending())
				listed.push_back(&buddy);

		std::stable_sort(listed.begin(), listed.end(), [](const Buddy* a, const Buddy* b) { return a->name < b->name; });
		std::stable_sort(listed.begin(), listed.end(),
			[](const Buddy* a, const Buddy* b) { return before(a->group, b->group); });

		std::vector<Group> groups;

		for (const Buddy* buddy : listed)
		{
			if (groups.empty() || groups.back().name != buddy->group)
				groups.push_back(Group{ buddy->group, {}, 0 });

			groups.back().members.push_back(buddy);

			if (buddy->online())
				groups.back().online++;
		}

		if (groups.empty() || groups.back().name != DEFAULT)
			groups.push_back(Group{ DEFAULT, {}, 0 });

		return groups;
	}

	std::vector<std::string> names(const std::vector<Buddy>& buddies)
	{
		std::vector<std::string> result;

		for (const Buddy& buddy : buddies)
			if (buddy.id != 0 && std::find(result.begin(), result.end(), buddy.group) == result.end())
				result.push_back(buddy.group);

		std::stable_sort(result.begin(), result.end(), before);

		return result;
	}

	Blocks::Blocks(std::set<int32_t> ids) : ids_(std::move(ids)) {}

	bool Blocks::blocked(int32_t id) const
	{
		return ids_.count(id) > 0;
	}

	void Blocks::set(int32_t id, bool blocked)
	{
		if (blocked)
			ids_.insert(id);
		else
			ids_.erase(id);
	}

	bool Blocks::all_blocked(const Group& group) const
	{
		return std::all_of(group.members.begin(), group.members.end(),
			[&](const Buddy* buddy) { return blocked(buddy->id); });
	}

	const std::set<int32_t>& Blocks::ids() const
	{
		return ids_;
	}

	bool away(const Buddy& buddy, const Blocks& blocks)
	{
		return blocks.blocked(buddy.id) || !buddy.online();
	}

	const char* tip(const Buddy& buddy, const Blocks& blocks, int32_t own_channel)
	{
		if (blocks.blocked(buddy.id))
			return "This user has been blocked.";

		if (!buddy.online())
			return "Double-click to send a note.";

		if (buddy.channel >= 0 && buddy.channel == own_channel)
			return "Double-click to send a whisper. (same channel)";

		return "Double-click to send a whisper.";
	}

	namespace
	{
		std::vector<std::string> split(const std::string& text, char separator)
		{
			std::vector<std::string> parts;
			size_t start = 0;

			while (start <= text.size())
			{
				size_t end = text.find(separator, start);

				if (end == std::string::npos)
					end = text.size();

				if (end > start)
					parts.push_back(text.substr(start, end - start));

				start = end + 1;
			}

			return parts;
		}

		// An "owner:id" pair, or false if the text is not one.
		bool parse_pair(const std::string& text, int32_t& owner, int32_t& id)
		{
			size_t colon = text.find(':');

			if (colon == std::string::npos)
				return false;

			try
			{
				owner = std::stoi(text.substr(0, colon));
				id = std::stoi(text.substr(colon + 1));
			}
			catch (const std::exception&)
			{
				return false;
			}

			return true;
		}
	}

	std::set<int32_t> parse_blocks(const std::string& text, int32_t owner)
	{
		std::set<int32_t> ids;

		for (const std::string& part : split(text, ','))
		{
			int32_t pair_owner = 0;
			int32_t id = 0;

			if (parse_pair(part, pair_owner, id) && pair_owner == owner)
				ids.insert(id);
		}

		return ids;
	}

	std::string format_blocks(const std::string& text, int32_t owner, const std::set<int32_t>& ids)
	{
		std::string result;

		auto add = [&](const std::string& part)
		{
			if (!result.empty())
				result += ',';

			result += part;
		};

		for (const std::string& part : split(text, ','))
		{
			int32_t pair_owner = 0;
			int32_t id = 0;

			if (parse_pair(part, pair_owner, id) && pair_owner != owner)
				add(part);
		}

		for (int32_t id : ids)
			add(std::to_string(owner) + ':' + std::to_string(id));

		return result;
	}

	std::set<std::string> parse_names(const std::string& text)
	{
		std::vector<std::string> parts = split(text, '\t');

		return std::set<std::string>(parts.begin(), parts.end());
	}

	std::string format_names(const std::set<std::string>& names)
	{
		std::string result;

		for (const std::string& name : names)
		{
			if (!result.empty())
				result += '\t';

			result += name;
		}

		return result;
	}

	Refusal check_group(const std::string& name, const std::string& original, bool renaming, bool any_checked,
						const std::vector<std::string>& groups)
	{
		if (name.empty())
			return Refusal::NO_NAME;

		if (!any_checked)
			return Refusal::NO_FRIENDS;

		if (renaming && name == original)
			return Refusal::NONE;

		if (std::find(groups.begin(), groups.end(), name) != groups.end())
			return Refusal::EXISTS;

		return Refusal::NONE;
	}

	std::vector<std::pair<std::string, std::string>> group_moves(const std::vector<Pick>& picks,
																 const std::string& name)
	{
		std::vector<std::pair<std::string, std::string>> moves;

		for (const Pick& pick : picks)
		{
			if (pick.checked)
				moves.emplace_back(pick.name, name);
			else if (pick.group == name)
				moves.emplace_back(pick.name, DEFAULT);
		}

		return moves;
	}

	Buttons buttons(const Buddy& picked, const Blocks& blocks)
	{
		bool blocked = blocks.blocked(picked.id);
		bool here = !away(picked, blocks);

		Buttons result;
		result.mod = true;
		result.remove = true;
		result.chat = here;
		result.whisper = here;
		result.message = !blocked;
		result.party = here;
		result.block = true;
		result.unblock = blocked;

		return result;
	}

	Buttons buttons(const Group& picked, const Blocks& blocks)
	{
		Buttons result;
		result.mod = picked.name != DEFAULT;
		result.remove = picked.name != DEFAULT;
		result.block = true;
		result.unblock = blocks.all_blocked(picked);

		return result;
	}
}
