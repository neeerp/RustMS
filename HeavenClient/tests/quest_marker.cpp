#include "../Character/QuestMarker.h"
#include <iostream>
#include <map>
#include <stdexcept>

using ms::QuestMarker;

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		std::vector<ms::QuestMarkerRule> chain;
		for (int16_t id = 1009; id <= 1015; ++id)
		{
			ms::QuestMarkerRule rule {id, 12101, 12101, {0}, {}, true};
			if (id > 1009) rule.prerequisites.emplace_back(id - 1, 2);
			chain.push_back(rule);
		}
		std::map<int16_t, int16_t> states;
		auto marker = [&](int32_t npc = 12101, int16_t job = 0)
		{
			return ms::select_quest_marker(npc, job, chain, [&](int16_t id)
			{
				auto found = states.find(id);
				return found == states.end() ? 0 : found->second;
			});
		};
		require(marker(10000) == QuestMarker::NONE, "Pio must not advertise Rain's quests");
		require(marker(12101, 100) == QuestMarker::NONE, "non-beginners must not see the bulb");
		for (int16_t id = 1009; id <= 1015; ++id)
		{
			require(marker() == QuestMarker::AVAILABLE, "each newly unlocked quiz needs a bulb");
			states[id] = 1;
			require(marker() == QuestMarker::READY, "accepted dialogue-only quiz needs a closed book");
			chain[id - 1009].completion_ready = false;
			require(marker() == QuestMarker::IN_PROGRESS, "unmet objectives need an open book");
			chain[id - 1009].completion_ready = true;
			states[id] = 2;
		}
		require(marker() == QuestMarker::NONE, "completed chain must clear its marker");
		states.clear();
		states[1010] = 1;
		require(marker() == QuestMarker::AVAILABLE, "started later quiz cannot bypass its prerequisite");
		chain.push_back({2000, 12101, 12101, {0}, {}, false});
		states[2000] = 1;
		require(marker() == QuestMarker::IN_PROGRESS, "active quest takes priority over an available quest");
		chain.push_back({2001, 999, 12101, {0}, {}, true});
		states[2001] = 1;
		require(marker() == QuestMarker::READY, "ready turn-in takes priority over other markers");
		states.clear();
		require(marker() == QuestMarker::AVAILABLE, "another character's progress must not affect availability");
		std::vector<ms::QuestMarkerRule> contact { {1032, 2102, 2001, {0}, {}, true, 10},
			{1033, 2001, 2102, {0}, {{1032, 2}}, true, 10} };
		auto contact_marker = [&](int32_t npc, int16_t level = 1) {
			return ms::select_quest_marker(npc, 0, contact, [&](int16_t id) { return states[id]; }, level);
		};
		require(contact_marker(2102) == QuestMarker::AVAILABLE, "Nina must offer the existing quest");
		require(contact_marker(2102, 11) == QuestMarker::NONE, "offer must honor maximum start level");
		auto status = [&](int16_t id) { return states[id]; };
		require(ms::quest_available(contact[0], 0, 1, status), "quest list must show eligible visits");
		require(!ms::quest_available(contact[0], 0, 11, status), "level filter must hide unavailable visits");
		require(ms::quest_available(contact[0], 0, 11, status, true), "all-levels view may show out-of-range visits");
		require(!ms::quest_available(contact[0], 100, 1, status, true), "all-levels view must still honor job requirements");
		require(!ms::quest_available(contact[1], 0, 1, status, true), "quest list must not bypass prerequisites");
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
		std::cout << "Quest marker transitions and priority passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
