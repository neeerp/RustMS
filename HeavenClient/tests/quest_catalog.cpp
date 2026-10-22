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

	// Pio takes back his two items and pays meso and EXP; only those are listed.
	if (pio->rewards.summary(0, false, false) !=
		"#w\r\n\r\n#fUI/UIWindow.img/QuestIcon/4/0#\r\n\r\n"
		"#fUI/UIWindow.img/QuestIcon/7/0# 200 Meso\r\n#fUI/UIWindow.img/QuestIcon/8/0# 100 exp\r\n\r\n#w")
	{
		std::cerr << "Quest 1008's rewards are not listed as expected\n";
		return 1;
	}

	// Todd gives beginners one of five hats.
	if (todd->rewards.summary(0, false, false).find("QuestIcon/5/0") == std::string::npos ||
		todd->rewards.summary(100, false, false).find("QuestIcon/5/0") != std::string::npos)
	{
		std::cerr << "Quest 1035's random hats are not listed as expected\n";
		return 1;
	}

	// The quest alert's flags and triggers.
	const ms::QuestInfo* blake = catalog.find(2292);
	const ms::QuestInfo* parwen = catalog.find(3318);
	const ms::QuestInfo* story = catalog.find(2313);

	if (!blake || !blake->auto_start || blake->auto_complete || !blake->normal_auto_start || !parwen ||
		parwen->normal_auto_start || parwen->field_enter != std::vector<int32_t>{ 261020401 } || !story ||
		!story->auto_complete || story->auto_start)
	{
		std::cerr << "The quest alert's flags did not parse as expected\n";
		return 1;
	}

	ms::QuestScript rain = catalog.script(1009, true);

	if (!rain.ask || rain.questions.at(0).wrong.count(1) != 1 || rain.questions.at(0).wrong.count(0) != 0)
	{
		std::cerr << "Rain's quiz did not parse as expected\n";
		return 1;
	}

	ms::QuestScript lana = catalog.script(2270, true);

	if (!lana.questions.at(0).flip || lana.stop("npc").empty())
	{
		std::cerr << "Lana's questions did not parse as expected\n";
		return 1;
	}

	// The ten party quests, which are never offered.
	const std::vector<ms::PartyQuest>& party = catalog.party_quests();

	if (party.size() != 10 || party.front().id != 1200 || party.back().id != 1302)
	{
		std::cerr << "Expected the party quests 1200-1206 and 1300-1302, got " << party.size() << '\n';
		return 1;
	}

	for (const ms::PartyQuest& quest : party)
	{
		if (catalog.find(quest.id) || quest.name.empty() || quest.min_level == 0 ||
			quest.mark.rfind("Map/MapHelper/mark/", 0) != 0)
		{
			std::cerr << "Party quest " << quest.id << " did not parse as expected\n";
			return 1;
		}
	}

	const ms::PartyQuest& kerning = party[1];
	const ms::PartyQuest& ellin = party[6];

	if (kerning.id != 1201 || kerning.name != "First Time Together" || kerning.min_level != 21 || kerning.max_level != 30 ||
		kerning.mark != "Map/MapHelper/mark/KerningParty" || !kerning.jobs.empty() || ellin.id != 1206 ||
		ellin.min_level != 45 || ellin.max_level != 55 || ellin.jobs.size() != 42)
	{
		std::cerr << "Party quests 1201 and 1206 did not parse as expected\n";
		return 1;
	}

	std::cout << "All " << catalog.quests().size() << " supported quests load\n";
	return 0;
}
