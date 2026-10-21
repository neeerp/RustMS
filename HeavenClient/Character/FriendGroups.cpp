#include "FriendGroups.h"

#include <algorithm>
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
