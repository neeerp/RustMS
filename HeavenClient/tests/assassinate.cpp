#include "../Gameplay/Combat/Assassinate.h"
#include "../Gameplay/Combat/Assaulter.h"

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
		using namespace ms::assassinate;

		require(refused(ID, false) && !refused(ID, true), "Assassinate goes only from Dark Sight");
		require(refused(DARK_SIGHT, true) && refused(4201005, true), "in Dark Sight nothing else goes");
		require(!refused(4201005, false) && !refused(0, true), "a regular attack always goes");
		require(!refused(14111002, true, true) && refused(NIGHT_SIGHT, true, true),
				"Vanish's owner uses anything in Dark Sight but Dark Sight");
		require(dark_sight(1411) == NIGHT_SIGHT && dark_sight(421) == DARK_SIGHT && dark_sight(900) == DARK_SIGHT,
				"the attack key cancels the job's Dark Sight");
		require(attack_ends(true, false) && !attack_ends(true, true) && !attack_ends(false, false),
				"the attack key ends Dark Sight, unless the player owns Vanish");
		require(recast(WIND_WALK, true) && !recast(WIND_WALK, false) && !recast(DARK_SIGHT, true),
				"Wind Walk isn't cast over itself");
		require(unseen(0, 0) == 100 && unseen(130, 0) == 130 && unseen(0, 103) == 103,
				"Wind Walk and Vanish make each line their percent");

		require(multiplier(-1, 12) == 0, "never in Dark Sight, nothing");
		require(multiplier(0, 12) == 1 && multiplier(2999, 12) == 1, "under 3 seconds, once");
		require(multiplier(3000, 12) == 2 && multiplier(6000, 12) == 3 && multiplier(9000, 12) == 4,
				"once more every 3 seconds");
		require(multiplier(12000, 12) == 5 && multiplier(60000, 12) == 5, "up to the skill's time");
		require(multiplier(60000, 6) == 3 && multiplier(60000, 9) == 4, "lower levels build up less");

		using namespace ms;

		require(assaulter::needs_mob(assaulter::ID), "Assaulter goes only at a mob");
		require(assaulter::needs_mob(5111004) && assaulter::needs_mob(15111001), "so do the Energy Drains");
		require(assaulter::needs_mob(5121007) && assaulter::needs_mob(15111004), "and the Barrages");
		require(!assaulter::needs_mob(0) && !assaulter::needs_mob(4201005) && !assaulter::needs_mob(ID),
				"the others swing at the air");
		require(assaulter::MOVE != assassinate::MOVE, "others tell the two dashes apart");

		std::cout << "assassinate tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}

	return 0;
}
