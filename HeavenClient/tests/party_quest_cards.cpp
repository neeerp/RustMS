#include "../Character/PartyQuests.h"

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
		using Ids = std::vector<int16_t>;

		auto ids = [](const std::vector<const PartyQuest*>& cards)
		{
			Ids result;

			for (const PartyQuest* quest : cards)
				result.push_back(quest->id);

			return result;
		};

		// As in Quest.nx: Kerning City's from 21, the Ariant contest also
		// from 21, the Goddess Tower and the second Carnival from 51, and the
		// Ellin forest from 45 for some jobs.
		std::vector<PartyQuest> quests = {
			{ 1201, "First Time Together", 21, 30, {}, "Map/MapHelper/mark/KerningParty" },
			{ 1203, "Remnants of Goddess", 51, 70, {}, "Map/MapHelper/mark/GoddessTower" },
			{ 1206, "Forest of Poison Haze", 45, 55, { 0, 100, 110 }, "Map/MapHelper/mark/EilnForest" },
			{ 1300, "Ariant Hunting Competition", 21, 30, {}, "Map/MapHelper/mark/AriantMatch" },
			{ 1302, "The 2nd Monster Carnival", 51, 70, {}, "Map/MapHelper/mark/Carnival" },
		};

		// One card for each quest starting at the new level.
		require(ids(party_quest_cards(quests, 0, 21)) == Ids{ 1201, 1300 }, "two cards at 21");
		require(ids(party_quest_cards(quests, 0, 51)) == Ids{ 1203, 1302 }, "two cards at 51");
		require(party_quest_cards(quests, 0, 22).empty(), "none at 22, within the range");
		require(party_quest_cards(quests, 0, 20).empty(), "none before the range");

		// A job list lets only its jobs in, and any GM.
		require(ids(party_quest_cards(quests, 110, 45)) == Ids{ 1206 }, "a listed job");
		require(party_quest_cards(quests, 200, 45).empty(), "a job not listed");
		require(ids(party_quest_cards(quests, 900, 45)) == Ids{ 1206 }, "a GM");
		require(ids(party_quest_cards(quests, 910, 45)) == Ids{ 1206 }, "a super GM");

		// The Available tab lists one from five levels below its lowest to its
		// highest, for the same jobs.
		const PartyQuest& kerning = quests[0];
		const PartyQuest& forest = quests[2];
		require(!party_quest_listed(kerning, 0, 15), "not six levels early");
		require(party_quest_listed(kerning, 0, 16), "five levels early");
		require(party_quest_listed(kerning, 0, 30), "at its highest");
		require(!party_quest_listed(kerning, 0, 31), "not past its highest");
		require(party_quest_listed(kerning, 0, 1, true), "any level with all levels");
		require(party_quest_listed(forest, 110, 45), "a listed job");
		require(!party_quest_listed(forest, 200, 45), "not a job not listed");
		require(!party_quest_listed(forest, 200, 45, true), "not a job not listed, all levels");
		require(party_quest_listed(forest, 900, 45), "a GM");
	}
	catch (const std::exception& e)
	{
		std::cerr << "party_quest_cards: " << e.what() << '\n';
		return 1;
	}

	std::cout << "party_quest_cards: ok\n";
	return 0;
}
