#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <numeric>
#include <utility>
#include <vector>

namespace ms
{
	// Which of a skill's hit animations each hit shows, for the skills that
	// have one per mob or per hit, and the order a melee attack's mobs are
	// hit in.
	namespace hit_art
	{
		constexpr int32_t BAND_OF_THIEVES = 4211004;
		constexpr int32_t DOUBLE_UPPERCUT = 5101003;

		// The first mob's hit is hit/0 and the others' hit/1 (0x75a53b): for
		// Corkscrew Blow, Arrow Bomb and three summons, Phoenix, Frostprey
		// and Gaviota.
		inline bool first_apart(int32_t skill)
		{
			return skill == 5101004 || skill == 15101003 || skill == 3101005 || skill == 3121006 ||
				   skill == 3221005 || skill == 5211002;
		}

		// Barrage shows its hit, hit0 and hit1 together at every hit, hit1 above the others
		// (0x6682ca).
		inline bool layered(int32_t skill)
		{
			return skill == 5121007 || skill == 15111004;
		}

		// Whether the skill's art is chosen here, not by its weapon.
		inline bool listed(int32_t skill)
		{
			return skill == BAND_OF_THIEVES || skill == DOUBLE_UPPERCUT || first_apart(skill);
		}

		// Band of Thieves's mobs: the first gets hit/0, and the others a thief
		// each from hit/1 to hit/x, shuffled by each mob's roll (0x75ab0b).
		inline std::vector<uint8_t> band(size_t mobs, int32_t x, const std::vector<int32_t> &rolls)
		{
			std::vector<uint8_t> arts(std::max<size_t>(static_cast<size_t>(std::max(x, 0)) + 1, mobs));
			std::iota(arts.begin(), arts.end(), static_cast<uint8_t>(0));

			if (x > 0)
				for (size_t i = 1; i < mobs && i <= rolls.size(); i++)
					std::swap(arts[i], arts[1 + rolls[i - 1] % x]);

			arts.resize(mobs);

			return arts;
		}

		// The art of the `hit`th hit on the `target`th mob.
		inline uint8_t of(int32_t skill, size_t target, size_t hit, const std::vector<uint8_t> &band_arts)
		{
			if (skill == BAND_OF_THIEVES)
				return target < band_arts.size() ? band_arts[target] : 0;

			if (skill == DOUBLE_UPPERCUT)
				return static_cast<uint8_t>(std::min<size_t>(hit, 1));

			return first_apart(skill) && target > 0 ? 1 : 0;
		}

		// A melee attack hits its mobs from the nearest along the way the user
		// faces, but for Band of Thieves and Spark (0x951ca2).
		inline bool sorts_mobs(int32_t skill)
		{
			return skill != BAND_OF_THIEVES && skill != 15111006;
		}

		// The client's exchange sort on where each mob is hit (0x953760).
		template <typename T>
		void sort_mobs(std::vector<std::pair<int32_t, T>> &mobs, bool right)
		{
			for (size_t i = 0; i + 1 < mobs.size(); i++)
				for (size_t j = i + 1; j < mobs.size(); j++)
					if (right ? mobs[i].first > mobs[j].first : mobs[i].first < mobs[j].first)
						std::swap(mobs[i], mobs[j]);
		}
	}
}
