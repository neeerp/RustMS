#include "../Character/SpResetRules.h"
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
		using namespace ms::sp_reset;

		constexpr int32_t POWER_STRIKE = 1001004, SLASH_BLAST = 1001005, IRON_BODY = 1001003;
		constexpr int32_t HP_RECOVERY = 1000000, SWORD_MASTERY = 1100000, SWORD_BOOSTER = 1101004;
		constexpr int32_t RAGE = 1101006, RUSH = 1121006;

		const std::unordered_map<int32_t, int32_t> none;
		const std::unordered_map<int32_t, int32_t> booster_reqs = { { SWORD_MASTERY, 5 } };
		auto reqs_of = [&](int32_t skill_id) -> const std::unordered_map<int32_t, int32_t>& {
			return skill_id == SWORD_BOOSTER ? booster_reqs : none;
		};

		require(advancement(0) == 1 && advancement(100) == 1 && advancement(110) == 2 && advancement(112) == 4,
			"advancements");
		require(job_path(112) == std::vector<int32_t>({ 100, 110, 111, 112 }), "a hero's path");
		require(job_path(1111) == std::vector<int32_t>({ 1100, 1110, 1111 }), "a Cygnus path");
		require(job_path(2112) == std::vector<int32_t>({ 2100, 2110, 2111, 2112 }), "an Aran's path");
		require(job_path(0).empty() && job_path(230) == std::vector<int32_t>({ 200, 230 }), "short paths");

		require(is_item(5050001) && is_item(5050004) && !is_item(5050000), "the four items");
		require(item_tab(5050001) == 0 && item_tab(5050004) == 3, "item tabs");
		require(may_use(100, 0) && may_use(110, 1) && !may_use(110, 2), "advanced far enough");
		require(!may_use(0, 0) && !may_use(900, 0) && may_use(1100, 0), "not beginners or GMs");

		require(skill_tab(job_path(112), RAGE) == 1 && skill_tab(job_path(112), 2001002) == -1, "skill tabs");
		require(has_master_level(RUSH) && !has_master_level(RAGE) && !has_master_level(2001002), "master levels");
		require(has_master_level(22170001) && has_master_level(22181002) && !has_master_level(22120001),
				"Evan master levels");

		// Level 30: 62 earned as a fighter, 61 of them as a warrior, all spent there.
		Character fighter{ 110, 30, 1, { { POWER_STRIKE, 20 }, { SLASH_BLAST, 20 }, { HP_RECOVERY, 20 },
										 { IRON_BODY, 1 } } };
		require(spare(fighter, 1) == 0, "nothing spare");
		require(may_take(fighter, POWER_STRIKE, 0, 0, reqs_of), "within the item's advancement");
		require(!may_take(fighter, POWER_STRIKE, 0, 1, reqs_of), "an earlier one needs spare SP");
		Character more = fighter;
		more.level = 31;
		more.levels[IRON_BODY] = 4;
		require(spare(more, 1) == 3 && may_take(more, POWER_STRIKE, 0, 1, reqs_of), "level 31's three");
		more.sp = 4;
		more.levels[IRON_BODY] = 1;
		require(spare(more, 1) == 0, "unspent SP isn't spare");

		Character swordsman{ 110, 35, 0, { { SWORD_MASTERY, 5 }, { SWORD_BOOSTER, 1 } } };
		require(!may_take(swordsman, SWORD_MASTERY, 1, 1, reqs_of), "Booster needs Mastery 5");
		require(!may_take(swordsman, RAGE, 1, 1, reqs_of), "nothing to take");
		swordsman.levels[SWORD_MASTERY] = 6;
		require(may_take(swordsman, SWORD_MASTERY, 1, 1, reqs_of), "6 is one more");
		swordsman.levels[SWORD_BOOSTER] = 0;
		swordsman.levels[SWORD_MASTERY] = 5;
		require(may_take(swordsman, SWORD_MASTERY, 1, 1, reqs_of), "a level 0 Booster needs nothing");

		require(may_give(swordsman, RAGE, SWORD_MASTERY, 0, 20, none), "up it goes");
		require(!may_give(swordsman, SWORD_MASTERY, SWORD_MASTERY, 0, 20, none), "not where it came from");
		swordsman.levels[SWORD_MASTERY] = 4;
		require(!may_give(swordsman, SWORD_BOOSTER, RAGE, 0, 20, booster_reqs), "Booster's req");
		swordsman.levels[RAGE] = 20;
		require(!may_give(swordsman, RAGE, SWORD_MASTERY, 0, 20, none), "full");

		Character hero{ 112, 120, 0, { { RUSH, 10 } } };
		require(!may_give(hero, RUSH, RAGE, 10, 30, none) && may_give(hero, RUSH, RAGE, 11, 30, none),
			"up to the master level");

		std::cout << "SP reset rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
