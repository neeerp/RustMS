#include "../Character/QuestMarker.h"
#include <iostream>
#include <map>
#include <set>
#include <string>
#include <stdexcept>

using ms::QuestMarker;

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

ms::QuestInfo quest(int16_t id, int32_t start_npc, int32_t end_npc,
	std::vector<std::pair<int16_t, int16_t>> prerequisites = {}, int16_t max_level = 32767)
{
	ms::QuestInfo info { id, "", start_npc, end_npc, {0} };
	info.prerequisites = std::move(prerequisites);
	info.max_level = max_level;
	return info;
}

int main()
{
	try
	{
		auto anyone = [](const ms::QuestInfo&) { return true; };
		std::vector<ms::QuestInfo> chain;
		for (int16_t id = 1009; id <= 1015; ++id)
			chain.push_back(id > 1009 ? quest(id, 12101, 12101, {{id - 1, 2}}) : quest(id, 12101, 12101));
		std::map<int16_t, int16_t> states;
		auto marker = [&](int32_t npc = 12101, int16_t job = 0)
		{
			return ms::select_quest_marker(npc, job, chain, [&](int16_t id)
			{
				auto found = states.find(id);
				return found == states.end() ? 0 : found->second;
			}, 1, [](int32_t) { return 0; }, [](int16_t) { return std::string(); }, anyone);
		};
		require(marker(10000) == QuestMarker::NONE, "Pio must not advertise Rain's quests");
		require(marker(12101, 100) == QuestMarker::NONE, "non-beginners must not see the bulb");
		for (int16_t id = 1009; id <= 1015; ++id)
		{
			require(marker() == QuestMarker::AVAILABLE, "each newly unlocked quiz needs a bulb");
			states[id] = 1;
			require(marker() == QuestMarker::READY, "accepted dialogue-only quiz needs a closed book");
			chain[id - 1009].end_items = {{4000000, 1}};
			require(marker() == QuestMarker::IN_PROGRESS, "unmet objectives need an open book");
			chain[id - 1009].end_items.clear();
			states[id] = 2;
		}
		require(marker() == QuestMarker::NONE, "completed chain must clear its marker");
		chain.push_back(quest(2000, 12101, 999));
		states[2000] = 1;
		require(marker() == QuestMarker::IN_PROGRESS, "an active quest shows once nothing is available");
		states[1015] = 0;
		require(marker() == QuestMarker::AVAILABLE, "an available quest outranks an active one");
		chain.push_back(quest(2001, 999, 12101));
		states[2001] = 1;
		require(marker() == QuestMarker::READY, "ready turn-in takes priority over other markers");
		states.clear();
		require(marker() == QuestMarker::AVAILABLE, "another character's progress must not affect availability");
		auto lone = [&](const ms::QuestInfo& info, int16_t state)
		{
			return ms::select_quest_marker(12101, 0, {info}, [&](int16_t) { return state; }, 1,
				[](int32_t) { return 0; }, [](int16_t) { return std::string(); }, anyone);
		};
		require(lone(quest(1250, 12101, 12101), 0) == QuestMarker::NONE, "party quests get no icon");
		require(lone(quest(2002, 12101, 0), 1) == QuestMarker::READY, "a quest without a finishing NPC is ready anywhere");
		std::vector<ms::QuestInfo> contact { quest(1032, 2102, 2001, {}, 10), quest(1033, 2001, 2102, {{1032, 2}}, 10) };
		auto contact_marker = [&](int32_t npc, int16_t level = 1) {
			return ms::select_quest_marker(npc, 0, contact, [&](int16_t id) { return states[id]; }, level,
				[](int32_t) { return 0; }, [](int16_t) { return std::string(); }, anyone);
		};
		require(contact_marker(2102) == QuestMarker::AVAILABLE, "Nina must offer the existing quest");
		require(contact_marker(2102, 11) == QuestMarker::NONE, "offer must honor maximum start level");
		auto status = [&](int16_t id) { return states[id]; };
		require(ms::quest_available(contact[0], 0, 1, status, anyone), "quest list must show eligible visits");
		require(!ms::quest_available(contact[0], 0, 11, status, anyone), "level filter must hide unavailable visits");
		require(ms::quest_available(contact[0], 0, 11, status, anyone, true), "all-levels view may show out-of-range visits");
		require(!ms::quest_available(contact[0], 100, 1, status, anyone, true), "all-levels view must still honor job requirements");
		require(!ms::quest_available(contact[1], 0, 1, status, anyone, true), "quest list must not bypass prerequisites");
		require(contact_marker(2001) == QuestMarker::NONE, "Sen must wait for acceptance");
		states[1032] = 1;
		require(contact_marker(2102) == QuestMarker::IN_PROGRESS, "source NPC must show an open book while waiting");
		require(contact_marker(2001, 11) == QuestMarker::READY, "destination NPC must allow an already started turn-in");
		states[1032] = 2;
		require(contact_marker(2001) == QuestMarker::AVAILABLE, "completed visit must unlock Sen's reply quest");
		states[1033] = 1;
		require(contact_marker(2102) == QuestMarker::READY, "returning to Nina must show a closed book");
		states[1033] = 2;
		require(contact_marker(2102) == QuestMarker::NONE, "unsupported item quest must not be advertised");
		// Todd's hunt is ready once the kill count and the item are in.
		ms::QuestInfo hunt = quest(1035, 2004, 2002);
		hunt.end_mobs = {{9300018, 1}};
		hunt.end_items = {{4031802, 1}};
		std::string progress = "000";
		int32_t shells = 0;
		auto hunt_marker = [&]() {
			return ms::select_quest_marker(2002, 0, {hunt}, [](int16_t) { return 1; }, 1,
				[&](int32_t) { return shells; }, [&](int16_t) { return progress; }, anyone);
		};
		require(hunt_marker() == QuestMarker::IN_PROGRESS, "no kills yet");
		shells = 1;
		require(hunt_marker() == QuestMarker::IN_PROGRESS, "the item alone is not enough");
		progress = "001";
		require(hunt_marker() == QuestMarker::READY, "kills and item complete the hunt");
		require(ms::decode_kills("003012") == std::vector<int16_t>({3, 12}), "progress decodes three digits per mob");
		// v83 checks the NPC, items, quests, then kills (FUN_00721d2c).
		hunt.end_prerequisites = {{1034, 2}};
		int16_t before = 1;
		auto stop = [&](int32_t npc) {
			return ms::completion_stop(hunt, npc, [&](int16_t id) { return id == 1034 ? before : 1; },
				[&](int32_t) { return shells; }, progress);
		};
		shells = 0;
		progress = "000";
		require(stop(2004) == ms::QuestStop::NPC, "the wrong NPC is told first");
		require(stop(2002) == ms::QuestStop::ITEM, "then missing items");
		shells = 1;
		require(stop(2002) == ms::QuestStop::QUEST, "then quests in the wrong state");
		before = 2;
		require(stop(2002) == ms::QuestStop::MOB, "then kills");
		progress = "001";
		require(stop(2002) == ms::QuestStop::NONE, "and then it can be completed");
		// Start requirements on what the player holds (FUN_00721163): worn
		// copies count once, 0 means none, and a negative count is a maximum.
		ms::QuestInfo mirror = quest(1000, 2101, 2101);
		mirror.held_items = {{1042003, 1}, {2010007, 0}, {4000381, -1}};
		std::map<int32_t, int32_t> held { {1042003, 1} };
		std::set<int32_t> skills;
		int16_t fame = 0;
		auto holds = [&](const ms::QuestInfo& info) {
			return ms::start_holdings_met(info, fame, [&](int32_t id) { return held[id]; },
				[&](int32_t id) { return skills.count(id) > 0; });
		};
		auto mirror_marker = [&]() {
			return ms::select_quest_marker(2101, 0, {mirror}, [](int16_t) { return 0; }, 1,
				[](int32_t) { return 0; }, [](int16_t) { return std::string(); }, holds);
		};
		require(mirror_marker() == QuestMarker::AVAILABLE, "held items allow the start");
		held[4000381] = 2;
		require(mirror_marker() == QuestMarker::NONE, "more than a negative count's bound blocks it");
		held[4000381] = 1;
		held[2010007] = 1;
		require(!holds(mirror), "an item that must not be held blocks it");
		held[2010007] = 0;
		held[1042003] = 0;
		require(!holds(mirror), "a missing item blocks it");
		ms::QuestInfo book = quest(6110, 1022002, 1022002);
		book.skills = {{1001004, true}, {1121006, false}};
		book.min_fame = 1;
		skills = {1001004};
		require(!holds(book), "fame below the minimum blocks it");
		fame = -1;
		require(holds(book), "fame compares unsigned, so negative fame passes");
		skills.insert(1121006);
		require(!holds(book), "a skill that must be lacked blocks it");
		skills = {};
		require(!holds(book), "a skill that must be had blocks it");
		// Pio (10000) lists a hand-in he can't take yet as in progress.
		std::vector<ms::QuestInfo> pio { quest(1008, 10000, 10000), quest(1020, 10000, 10000),
			quest(1021, 999, 10000), quest(1022, 10000, 999), quest(1023, 10000, 10000), quest(1250, 10000, 10000) };
		std::map<int16_t, int16_t> pio_states { {1008, 1}, {1021, 1}, {1022, 1}, {1023, 2}, {1250, 1} };
		bool items = false;
		auto menu = [&]() {
			return ms::select_quest_menu(10000, pio, [&](int16_t id) { return pio_states[id]; },
				[&](const ms::QuestInfo& info) { return ms::quest_available(info, 0, 1, [&](int16_t id) { return pio_states[id]; }, anyone); },
				[&](const ms::QuestInfo& info) { return info.end_npc == 10000 && (info.id != 1008 || items); });
		};
		ms::QuestMenu listed = menu();
		require(listed.completable == std::vector<int16_t>({1021}), "a started quest he can take is completable");
		require(listed.available == std::vector<int16_t>({1020}), "an unstarted quest he gives is available");
		require(listed.in_progress == std::vector<int16_t>({1008, 1022}), "the rest of his started quests are in progress");
		require(listed.quests == 6 && listed.finished == 1, "all his quests count, the party quest too, and one is done");
		items = true;
		require(menu().completable == std::vector<int16_t>({1008, 1021}), "the hand-in moves up once the items are in");
		pio_states = {};
		pio.resize(1);
		require(!menu().empty() && ms::select_quest_menu(10000, {}, [](int16_t) { return 0; },
			[](const ms::QuestInfo&) { return true; }, [](const ms::QuestInfo&) { return true; }).empty(), "no quests, no menu");
		ms::QuestInfo job = quest(1400, 1012100, 1012100);
		job.name = "The Path of a Bowman";
		require(ms::quest_menu_line(3, job, false, false) == "#d#L3# The Path of a Bowman#l#k\r\n", "a line names the quest");
		job.type = "[Job Adv.]";
		require(ms::quest_menu_line(3, job, true, false) ==
			"#d#L3# #r#e[Job Adv.]#d#nThe Path of a Bowman (In Progress)#l#k\r\n", "after its type, in red");
		require(ms::quest_menu_line(3, job, true, true).find(" (Pre-completion enabled)#l") != std::string::npos,
			"and says when it can be completed");
		std::cout << "Quest marker transitions and priority passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
