#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A party quest's entry in Quest.nx (ids 1200-1399): information only,
	// with no end NPC and no rewards. v83 shows a "Party Quest" card for it
	// on reaching its lowest level.
	struct PartyQuest
	{
		int16_t id;
		std::string name;
		int16_t min_level;
		int16_t max_level = 32767;
		// Empty for any job.
		std::vector<int16_t> jobs;
		// PQuest.img/<id>/mark: a path into Map.nx, such as
		// "Map/MapHelper/mark/KerningParty".
		std::string mark;
	};

	// The party quests whose card a player of `job` gets on reaching `level`:
	// those whose lowest level it is, for that job or any, and every one for a
	// GM. Neither the highest level nor the quest's state counts
	// (FUN_00a1fb52, a1fde1-a1ff21).
	std::vector<const PartyQuest*> party_quest_cards(const std::vector<PartyQuest>& quests, int16_t job, int16_t level);

	// Whether the Quest Log's Available tab lists `quest`, if never started,
	// for a player of `job` at `level`: from five levels below its lowest
	// (FUN_00721163, 7212e9-72131a) to its highest, for that job or any, and
	// for a GM. With `all_levels`, at any level.
	bool party_quest_listed(const PartyQuest& quest, int16_t job, int16_t level, bool all_levels = false);
}
