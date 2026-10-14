#include "../Character/QuestCatalog.h"

#include <nlnx/file.hpp>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

// Every quest the server supports (game-data/supported_quests.txt) must load
// from Quest.nx.
int main()
{
	const char* override_path = std::getenv("RUSTMS_QUEST_NX_PATH");
	std::filesystem::path quest_nx = override_path ? override_path : DEFAULT_QUEST_NX;

	if (!std::filesystem::exists(quest_nx))
	{
		const char* required = std::getenv("RUSTMS_REQUIRE_ASSETS");

		if (required && std::string(required) == "1")
		{
			std::cerr << "Quest.nx not found at " << quest_nx << '\n';
			return 1;
		}

		std::cout << "Skipping: Quest.nx not found at " << quest_nx << '\n';
		return 0;
	}

	nl::file file(quest_nx.string());
	std::ifstream list(SUPPORTED_QUESTS);
	ms::QuestCatalog catalog(file, list);

	for (const ms::QuestInfo& quest : catalog.quests())
	{
		if (!quest.start_npc || !quest.end_npc || quest.name.empty())
		{
			std::cerr << "Quest " << quest.id << " is missing its NPCs or name\n";
			return 1;
		}
	}

	// The beginner chain must parse fully.
	const ms::QuestInfo* sen = catalog.find(1033);

	if (!sen || sen->prerequisites.size() != 1 || sen->prerequisites[0].first != 1032 || sen->next_quest != 1034)
	{
		std::cerr << "Quest 1033 did not parse as expected\n";
		return 1;
	}

	const ms::QuestInfo* todd = catalog.find(1035);

	if (!todd || todd->end_mobs.size() != 1 || todd->end_mobs[0] != std::make_pair(9300018, int16_t(1)))
	{
		std::cerr << "Quest 1035's hunt did not parse as expected\n";
		return 1;
	}

	// Pio's quest has no completion NPC; it goes back to Pio.
	const ms::QuestInfo* pio = catalog.find(1008);

	if (!pio || pio->start_npc != 10000 || pio->end_npc != 10000)
	{
		std::cerr << "Quest 1008 is not handed in to Pio\n";
		return 1;
	}

	ms::QuestScript rain = catalog.script(1009, true);

	if (rain.answers.at(0) != 0 || rain.wrong.at(0).count(1) != 1)
	{
		std::cerr << "Rain's first quiz answer did not parse as expected\n";
		return 1;
	}

	std::cout << "All " << catalog.quests().size() << " supported quests load\n";
	return 0;
}
