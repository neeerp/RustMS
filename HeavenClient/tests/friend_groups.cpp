#include "../Character/FriendGroups.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::FriendGroups;

		auto buddy = [](int32_t id, const char* name, const char* group, int32_t channel, uint8_t flag = 0)
		{
			Buddy b;
			b.id = id;
			b.name = name;
			b.group = group;
			b.channel = channel;
			b.flag = flag;
			return b;
		};

		require(before("Mates", DEFAULT) && !before(DEFAULT, "Mates"), "the Default Group goes after the rest");
		require(before("Alpha", "Beta") && !before("Beta", "Alpha"), "the rest by name");
		require(before("Zed", "alpha"), "as strcmp has it, capitals first");
		require(!before(DEFAULT, DEFAULT) && !before("Mates", "Mates"), "a group isn't before itself");

		std::vector<Buddy> none;
		auto empty = arrange(none);
		require(empty.size() == 1 && empty[0].name == DEFAULT && empty[0].members.empty(),
				"an empty list still shows the Default Group");

		std::vector<Buddy> list = {
			buddy(1, "Zoe", "Default Group", 0),
			buddy(2, "Amy", "Work", -1),
			buddy(3, "Bob", "Default Group", -1),
			buddy(4, "Cat", "Mates", 2),
			buddy(5, "Ann", "Work", 1),
			buddy(6, "Asker", "Mates", 0, 1),
		};

		auto groups = arrange(list);
		require(groups.size() == 3, "one group each, the request left out");
		require(groups[0].name == "Mates" && groups[1].name == "Work" && groups[2].name == DEFAULT,
				"groups by name, Default last");
		require(groups[0].members.size() == 1 && groups[0].members[0]->id == 4, "a request isn't shown");
		require(groups[1].members[0]->name == "Amy" && groups[1].members[1]->name == "Ann", "members by name");
		require(groups[2].members[0]->name == "Bob" && groups[2].members[1]->name == "Zoe", "in the Default Group too");
		require(groups[1].online == 1 && groups[2].online == 1, "and online counted per group");

		std::vector<Buddy> no_default = { buddy(1, "Zoe", "Work", 0) };
		auto added = arrange(no_default);
		require(added.size() == 2 && added[1].name == DEFAULT && added[1].members.empty(),
				"an empty Default Group is added after the rest");

		auto offered = names(list);
		require((offered == std::vector<std::string>{ "Mates", "Work", DEFAULT }), "each group once, Default last");

		Blocks blocks;
		require(blocks.all_blocked(Group{ "Empty", {}, 0 }), "an empty group counts as blocked");
		require(!blocks.all_blocked(groups[1]), "a group with an unblocked member isn't");
		blocks.set(2, true);
		require(!blocks.all_blocked(groups[1]), "nor with only one of two blocked");
		blocks.set(5, true);
		require(blocks.all_blocked(groups[1]), "but is with both");
		blocks.set(5, false);
		require(!blocks.blocked(5) && blocks.blocked(2), "unblocking takes one off");

		require(!away(list[4], blocks) && away(list[1], blocks), "online counts by channel");
		blocks.set(5, true);
		require(away(list[4], blocks), "and a blocked buddy is away");
		blocks.set(5, false);

		require(std::string(tip(list[0], blocks, 0)) == "Double-click to send a whisper. (same channel)",
				"a whisper tip on the player's channel");
		require(std::string(tip(list[4], blocks, 0)) == "Double-click to send a whisper.", "elsewhere");
		require(std::string(tip(list[2], blocks, 0)) == "Double-click to send a note.", "a note to one offline");
		require(std::string(tip(list[2], blocks, -1)) == "Double-click to send a note.",
				"offline matches no channel");
		require(std::string(tip(list[1], blocks, 0)) == "This user has been blocked.", "blocked though offline");
		blocks.set(1, true);
		require(std::string(tip(list[0], blocks, 0)) == "This user has been blocked.", "blocked first");
		blocks.set(1, false);

		auto online = buttons(list[4], blocks);
		require(online.mod && online.remove && online.chat && online.whisper && online.message && online.party
				&& online.block && !online.unblock, "an online buddy has every button");

		auto offline = buttons(list[2], blocks);
		require(offline.mod && offline.remove && !offline.chat && !offline.whisper && offline.message
				&& !offline.party && offline.block && !offline.unblock, "an offline one no chat, whisper or party");

		blocks.set(5, true);
		auto blocked = buttons(list[4], blocks);
		require(!blocked.chat && !blocked.whisper && !blocked.party && !blocked.message && blocked.block
				&& blocked.unblock, "a blocked one no message either, and UNBLOCK");

		auto group = buttons(groups[0], blocks);
		require(group.mod && group.remove && !group.chat && !group.whisper && !group.message && !group.party
				&& group.block && !group.unblock, "a group: edit, delete and block");

		auto fallback = buttons(groups[2], blocks);
		require(!fallback.mod && !fallback.remove && fallback.block, "the Default Group can't be edited or deleted");

		auto work = buttons(groups[1], blocks);
		require(work.unblock, "a group all blocked reads UNBLOCK");

		require(parse_blocks("", 1).empty(), "no blocks at first");
		require(parse_blocks("1:5,2:6,1:7,junk,3:", 1) == std::set<int32_t>{ 5, 7 },
				"each character's own blocks, and nothing malformed");
		require(format_blocks("1:5,2:6", 1, { 7, 8 }) == "2:6,1:7,1:8", "writing one's blocks keeps the others'");
		require(format_blocks("1:5", 1, {}) == "", "and an empty set clears them");

		require(parse_names("").empty(), "nothing folded at first");
		require(parse_names("Mates\tWork group") == std::set<std::string>{ "Mates", "Work group" },
				"names may hold spaces");
		require(format_names({ "Work group", "Mates" }) == "Mates\tWork group", "and are written in order");

		std::vector<std::string> taken = { "Mates", "Work", DEFAULT };
		require(check_group("", "Mates", true, false, taken) == Refusal::NO_NAME, "a group needs a name first");
		require(check_group("Pals", "", false, false, taken) == Refusal::NO_FRIENDS, "then a buddy checked");
		require(check_group("Mates", "Mates", true, true, taken) == Refusal::NONE, "keeping a group's name is fine");
		require(check_group("Work", "Mates", true, true, taken) == Refusal::EXISTS, "taking another's is not");
		require(check_group("Mates", "", false, true, taken) == Refusal::EXISTS, "nor is a new one taking it");
		require(check_group("Pals", "Mates", true, true, taken) == Refusal::NONE, "a fresh name is");

		using Moves = std::vector<std::pair<std::string, std::string>>;
		std::vector<Pick> picks = { { "Amy", "Mates", true }, { "Bob", "Mates", false }, { "Cat", "Pals", false },
									{ "Dan", "Work", true } };
		require(group_moves(picks, "Pals") == Moves{ { "Amy", "Pals" }, { "Cat", DEFAULT }, { "Dan", "Pals" } },
				"checked buddies join; unchecked ones of that name leave for the Default Group");
		require(group_moves(picks, "Mates") == Moves{ { "Amy", "Mates" }, { "Bob", DEFAULT }, { "Dan", "Mates" } },
				"which is how unchecking a member removes them");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
