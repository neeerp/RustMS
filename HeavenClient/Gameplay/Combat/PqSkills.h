#pragma once

#include <cstdint>

namespace ms
{
	// The secret skills every beginner line has at level 1 with no skill record (FUN_007616f6), each only in
	// its own maps: Bamboo Rain (1009), Invincibility (1010) and Power Explosion (1011) in the Dojo, Rage of
	// Pharaoh (1020) in the Pyramid.
	namespace pq_skills
	{
		enum class Kind
		{
			NONE,
			BAMBOO_RAIN,
			DOJO_BUFF,
			RAGE_OF_PHARAOH
		};

		constexpr int32_t INVINCIBILITY = 1010;

		// The field types they need (FUN_00587f5f).
		constexpr int32_t FIELD_DOJO = 14;
		constexpr int32_t FIELD_DOJO_2 = 18;
		constexpr int32_t FIELD_PYRAMID = 23;

		// Which of them a skill is, for the beginner, Noblesse, Aran and Evan copies.
		inline Kind kind(int32_t skill)
		{
			int32_t base = skill / 10000;

			if (base != 0 && base != 1000 && base != 2000 && base != 2001)
				return Kind::NONE;

			switch (skill % 10000)
			{
				case 1009:
					return Kind::BAMBOO_RAIN;
				case 1010:
				case 1011:
					return Kind::DOJO_BUFF;
				case 1020:
					return Kind::RAGE_OF_PHARAOH;
				default:
					return Kind::NONE;
			}
		}

		// Whether it may be used on a map of this field type; any other skill may (FUN_00587f5f).
		inline bool allowed(Kind kind, int32_t fieldtype)
		{
			switch (kind)
			{
				case Kind::BAMBOO_RAIN:
				case Kind::DOJO_BUFF:
					return fieldtype == FIELD_DOJO || fieldtype == FIELD_DOJO_2;
				case Kind::RAGE_OF_PHARAOH:
					return fieldtype == FIELD_PYRAMID;
				default:
					return true;
			}
		}

		// Whether a ride or a transformation refuses it: only the attacks (FUN_00587f5f, 0x9673cf).
		inline bool refused(Kind kind, bool riding, bool morphed)
		{
			bool attack = kind == Kind::BAMBOO_RAIN || kind == Kind::RAGE_OF_PHARAOH;

			return attack && (riding || morphed);
		}

		// What each line of the attack takes from a mob: all its max HP, or 30% of a boss's for Bamboo Rain,
		// with no roll and no critical (0x78e3dd, 0x7905a5); -1 for any other skill.
		inline int32_t strike(Kind kind, int32_t maxhp, bool boss)
		{
			switch (kind)
			{
				case Kind::BAMBOO_RAIN:
					return boss ? static_cast<int32_t>(maxhp * 0.3) : maxhp;
				case Kind::RAGE_OF_PHARAOH:
					return maxhp;
				default:
					return -1;
			}
		}

		// The job's own copy of a beginner skill: the Evan line's is 2001's (FUN_00943104).
		inline int32_t family(int32_t job, int32_t skill)
		{
			int32_t base = job / 100 == 22 || job == 2001 ? 2001 : job / 1000 * 1000;

			return base * 10000 + skill;
		}
	}
}
