#include "../Character/ApResetRules.h"
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
		using namespace ms::ap_reset;

		// A level 30 fighter: 20 + 150 earned, 160 held, so 10 AP are in Max HP or MP.
		Stats fighter{ 110, 30, 120, 30, 4, 4, 2, 1500, 200, 4 };

		require(may_use(0) && may_use(1100) && may_use(522) && !may_use(900), "all but GMs");
		require(branch(1111) == 1 && branch(232) == 2, "Cygnus by their branch");

		require(has_hp_mp_ap(fighter), "AP in HP or MP");
		Stats none = fighter;
		none.ap = 12;
		require(!has_hp_mp_ap(none) && !may_take(none, HP) && !may_take(none, MP), "none there");
		require(may_take(fighter, HP) && may_take(fighter, MP), "HP and MP can give one back");

		// 24 * 30 + 418 = 1138.
		Stats low = fighter;
		low.max_hp = 1138 + 53;
		require(!may_take(low, HP), "HP keeps the job's least");
		low.max_hp = 1138 + 54;
		require(may_take(low, HP), "down to it");

		require(may_take(fighter, STR) && may_take(fighter, DEX), "main stats above their least");
		require(!may_take(fighter, INT) && !may_take(fighter, LUK), "4 stays");
		Stats weak = fighter;
		weak.str = 35;
		require(!may_take(weak, STR), "a warrior keeps 35 STR");
		require(stat_floor(300, DEX) == 25 && stat_floor(500, DEX) == 20 && stat_floor(200, INT) == 20,
			"each job's requirement");

		require(mp_loss(230, 400) == 60 && mp_loss(200, 4) == 30, "a magician's follows INT");
		require(loss(fighter, HP) == 54 && loss(fighter, MP) == 4 && loss(fighter, LUK) == 1, "what leaves");
		require(gain(fighter, HP) == 20 && gain(fighter, MP) == 2 && gain(fighter, STR) == 1, "what arrives");

		require(!may_give(fighter, DEX, DEX), "not where it came from");
		Stats full = fighter;
		full.str = 999;
		require(!may_give(full, STR, DEX) && may_give(full, LUK, DEX), "999 is full");
		full.max_hp = 30000 - 40;
		require(may_give(full, HP, DEX), "two gains still fit");
		full.max_hp = 30000 - 39;
		require(!may_give(full, HP, DEX), "v83 counts the gain twice");

		require(min_mp(0, 10) == 95 && min_hp(522, 120) == 22 * 120 + 338 && min_hp(900, 200) == 0, "tables");

		std::cout << "AP reset rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
