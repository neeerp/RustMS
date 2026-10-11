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
		std::cout << "Quest marker transitions and priority passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
