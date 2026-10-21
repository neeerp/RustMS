#pragma once

#include "BuddyList.h"

#include <cstdint>
#include <set>
#include <string>
#include <utility>
#include <vector>

// The Friend tab's groups, as v83 arranges and acts on them (report b015). A group is only the
// name each buddy carries; the server keeps nothing else.
namespace ms::FriendGroups
{
	// Where buddies go unless put elsewhere; always shown, always last, and never renamed or
	// deleted.
	extern const std::string DEFAULT;

	// v83's group order (FUN_00921c15): by name, but the Default Group after every other.
	bool before(const std::string& group, const std::string& other);

	struct Group
	{
		std::string name;
		// By name.
		std::vector<const Buddy*> members;
		size_t online = 0;
	};

	// The list as the tab shows it (FUN_00909029): requests left out, groups in v83's order, and an
	// empty Default Group at the end if nobody is in it.
	std::vector<Group> arrange(const std::vector<Buddy>& buddies);

	// The groups the Add Friend and Group Whisper dialogs offer (FUN_00a3fc85): each buddy's, once,
	// in v83's order.
	std::vector<std::string> names(const std::vector<Buddy>& buddies);

	// The buddies the player has blocked: their chat is not heard, and they count as offline. v83
	// keeps these in the client's own settings and tells the server nothing.
	class Blocks
	{
	public:
		Blocks() = default;
		explicit Blocks(std::set<int32_t> ids);

		bool blocked(int32_t id) const;
		void set(int32_t id, bool blocked);
		// Whether every member is blocked, as for a group (FUN_0091db1b); an empty group is.
		bool all_blocked(const Group& group) const;
		const std::set<int32_t>& ids() const;

	private:
		std::set<int32_t> ids_;
	};

	// The settings' list of blocks, "owner:id" pairs split by commas. v83 keeps them per character
	// (report b020), so each owner's are read and written apart from the others'.
	std::set<int32_t> parse_blocks(const std::string& text, int32_t owner);
	std::string format_blocks(const std::string& text, int32_t owner, const std::set<int32_t>& ids);

	// The settings' list of groups folded, split by tabs, which no name holds.
	std::set<std::string> parse_names(const std::string& text);
	std::string format_names(const std::set<std::string>& names);

	// One buddy in the Edit Group dialog (FUN_0080ee78).
	struct Pick
	{
		std::string name;
		std::string group;
		bool checked;
	};

	// The Edit Group dialog's checks on OK (FUN_0080f93e), in v83's order. any_checked is only over the
	// rows in view, as v83 tests its 7 checkboxes and not the list behind them.
	enum class Refusal
	{
		NONE,
		NO_NAME,
		NO_FRIENDS,
		EXISTS
	};

	Refusal check_group(const std::string& name, const std::string& original, bool renaming, bool any_checked,
						const std::vector<std::string>& groups);

	// The adds that carry out the dialog (0x90af2a): each buddy checked goes to the group named, and
	// each unchecked one already in a group of that name goes to the Default Group.
	std::vector<std::pair<std::string, std::string>> group_moves(const std::vector<Pick>& picks,
																 const std::string& name);

	// v83's offline class (FUN_00908fc4): offline, or blocked.
	bool away(const Buddy& buddy, const Blocks& blocks);

	// Which of the buttons under the list are enabled, and which way BLOCK reads (FUN_0091dba9).
	// ADD FRIEND, ADD GROUP, GROUP WHISPER and SHOW ONLINE always are.
	struct Buttons
	{
		bool mod = false;
		bool remove = false;
		bool chat = false;
		bool whisper = false;
		bool message = false;
		bool party = false;
		bool block = false;
		bool unblock = false;
	};

	// With a buddy picked.
	Buttons buttons(const Buddy& picked, const Blocks& blocks);
	// With a group's header picked.
	Buttons buttons(const Group& picked, const Blocks& blocks);
}
