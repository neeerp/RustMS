#include "../Gameplay/Combat/HitArt.h"
#include <iostream>
#include <set>
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
		using hit_art::band;
		using hit_art::of;

		auto arts = band(5, 4, { 3, 2, 1, 0 });
		std::set<uint8_t> thieves(arts.begin() + 1, arts.end());

		require(arts[0] == 0, "the first mob gets hit/0");
		require(thieves == std::set<uint8_t>({ 1, 2, 3, 4 }), "the others get a thief each");
		require(band(3, 4, { 0, 0, 0 }) == std::vector<uint8_t>({ 0, 2, 1 }), "each roll swaps with the first thief");
		require(band(2, 1, { 7 }) == std::vector<uint8_t>({ 0, 1 }), "level 1 has only one thief");
		require(band(1, 4, { 3 }) == std::vector<uint8_t>({ 0 }), "one mob, hit/0");
		require(band(0, 4, {}).empty(), "no mobs");

		require(of(4211004, 1, 0, { 0, 3 }) == 3, "Band of Thieves's art goes by mob");
		require(of(5101004, 0, 0, {}) == 0 && of(5101004, 2, 0, {}) == 1, "Corkscrew Blow's first mob apart");
		require(of(3101005, 1, 0, {}) == 1, "Arrow Bomb's too");
		require(of(3121006, 0, 0, {}) == 0 && of(3121006, 3, 0, {}) == 1, "Phoenix's too");
		require(of(3221005, 1, 0, {}) == 1 && of(5211002, 1, 0, {}) == 1, "Frostprey's and Gaviota's too");
		require(of(3111005, 1, 0, {}) == 0, "but not Silver Hawk's, which has only hit/0");
		require(of(5101003, 0, 1, {}) == 1 && of(5101003, 0, 0, {}) == 0, "Double Uppercut's goes by hit");
		require(!hit_art::listed(1001005), "others go by weapon");

		std::vector<std::pair<int32_t, int32_t>> mobs = { { 30, 1 }, { 10, 2 }, { 20, 3 } };
		hit_art::sort_mobs(mobs, true);
		require(mobs[0].second == 2 && mobs[2].second == 1, "facing right, the lowest x first");
		hit_art::sort_mobs(mobs, false);
		require(mobs[0].second == 1 && mobs[2].second == 2, "facing left, the highest");
		require(!hit_art::sorts_mobs(4211004) && !hit_art::sorts_mobs(15111006) && hit_art::sorts_mobs(0), "all melee but two");
	} catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}
	return 0;
}
