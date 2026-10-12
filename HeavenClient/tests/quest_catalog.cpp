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

	ms::QuestScript rain = catalog.script(1009, true);

	if (rain.answers.at(0) != 0 || rain.wrong.at(0).count(1) != 1)
	{
		std::cerr << "Rain's first quiz answer did not parse as expected\n";
		return 1;
	}

	std::cout << "All " << catalog.quests().size() << " supported quests load\n";
	return 0;
}
