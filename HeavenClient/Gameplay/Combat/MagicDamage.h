#pragma once

#include <array>
#include <cstdint>
#include <string>

namespace ms
{
	// How a spell hits a monster, as the MapleRoyals client rolls it
	// (CalcDamage::MDamage, FUN_00791617). Each roll is a fraction in [0, 1)
	// that picks a value between two bounds.
	namespace magic_damage
	{
		// The elements, in the client's order (FUN_0075ee67): physical, ice,
		// fire, lightning, poison, holy, dark, and undead.
		constexpr size_t ELEMENTS = 8;

		// What a monster's element does to a hit of it.
		enum Attribute : uint8_t
		{
			NORMAL,
			IMMUNE,
			STRONG,
			WEAK
		};

		using Attributes = std::array<uint8_t, ELEMENTS>;

		// The element a letter of `elemAttr` names, in either case.
		inline uint8_t element(char letter)
		{
			switch (letter)
			{
			case 'I':
			case 'i':
				return 1;
			case 'F':
			case 'f':
				return 2;
			case 'L':
			case 'l':
				return 3;
			case 'S':
			case 's':
				return 4;
			case 'H':
			case 'h':
				return 5;
			case 'D':
			case 'd':
				return 6;
			case 'U':
			case 'u':
				return 7;
			default:
				return 0;
			}
		}

		// A monster's attributes from its `elemAttr`, pairs of an element
		// letter and an attribute digit such as "F2I3".
		inline Attributes attributes(const std::string &elemattr)
		{
			Attributes result = {};

			for (size_t i = 0; i + 1 < elemattr.size(); i += 2)
			{
				int attribute = elemattr[i + 1] - '0';

				if (attribute >= NORMAL && attribute <= WEAK)
					result[element(elemattr[i])] = static_cast<uint8_t>(attribute);
			}

			return result;
		}

		// The player's magic: INT and magic attack, at most 1999.
		inline int32_t magic(int32_t intl, int32_t matk)
		{
			int32_t total = intl + matk;

			if (total < 0)
				return 0;

			return total < 1999 ? total : 1999;
		}

		inline double between(double low, double high, double roll)
		{
			return low + (high - low) * roll;
		}

		// The magic accuracy, lower against monsters above the player.
		inline double accuracy(int32_t intl, int32_t luk, int32_t playerlevel, int32_t moblevel)
		{
			int32_t above = moblevel > playerlevel ? moblevel - playerlevel : 0;

			return (luk / 10 + intl / 10) * 5 * 100.0 / (above * 10.0 + 255.0);
		}

		// Whether a spell lands: a roll between half and 120% of the accuracy
		// must reach the monster's avoid, which counts at most 999.
		inline bool hits(double accuracy, int32_t avoid, double roll)
		{
			if (avoid > 999)
				avoid = 999;

			return between(accuracy * 0.5, accuracy * 1.2, roll) >= avoid;
		}

		// The damage before the monster's defense. The spell's mastery puts the
		// lowest magic roll at (mastery * 5 + 10) * 0.9% of the magic.
		inline double base(int32_t magic, int32_t intl, int32_t mastery, int32_t spell, double roll)
		{
			double low = magic * (mastery * 5 + 10) * 0.009;
			double rolled = between(low, magic, roll);

			return (rolled * 3.3 + magic * 0.058 * magic * 0.058 + intl * 0.5) * spell * 0.01;
		}

		// A hit of an element on a monster with the given attribute
		// (FUN_00790782).
		inline double attribute(double damage, uint8_t attribute)
		{
			switch (attribute)
			{
			case IMMUNE:
				return 0.0;
			case STRONG:
				return damage * 0.5;
			case WEAK:
				return damage * 1.5 < 199999.0 ? damage * 1.5 : 199999.0;
			default:
				return damage;
			}
		}

		// A skill's hit after the monster's attributes (FUN_00790660). Element
		// Composition hits with half its damage in each of its two elements.
		inline double elemental(double damage, int32_t skill, uint8_t element, const Attributes &attributes)
		{
			switch (skill)
			{
			case 2111006:
				return attribute(damage * 0.5, attributes[4]) + attribute(damage * 0.5, attributes[2]);
			case 2211006:
				return attribute(damage * 0.5, attributes[3]) + attribute(damage * 0.5, attributes[1]);
			default:
				return attribute(damage, attributes[element % ELEMENTS]);
			}
		}

		// The monster's magic defense takes 50% to 60% of itself off the hit;
		// it counts at most 1999.
		inline double defend(double damage, int32_t mdef, double roll)
		{
			if (mdef < 0)
				mdef = 0;

			if (mdef > 1999)
				mdef = 1999;

			return damage - between(mdef * 0.5, mdef * 0.6, roll);
		}

		// Whether a spell gets through a Magic Immunity: a roll of 0 to 99
		// must be at most the player's chance to pierce it, so 1 in 100
		// does with none. One that doesn't hits for 1.
		inline bool pierces(int32_t roll, int32_t chance)
		{
			return roll <= chance;
		}

		// A percent of the hit: Magic Guard Up's n, or the weapon's for
		// the element.
		inline double percent(double damage, int32_t percent)
		{
			return percent * damage * 0.01;
		}

		// Magic Guard Up leaves n% of the hit after the defense.
		inline double guarded(double damage, int32_t guard)
		{
			return guard ? percent(damage, guard) : damage;
		}

		// Weakness makes the hit n% harder.
		inline double weakened(double damage, int32_t weakness)
		{
			return weakness ? percent(damage, weakness + 100) : damage;
		}

		// The number shown: between 1 and 199999, rounded down.
		inline int32_t finish(double damage)
		{
			if (damage < 1.0)
				return 1;

			if (damage > 199999.0)
				return 199999;

			return static_cast<int32_t>(damage);
		}
	}
}
