#pragma once

#include <cstdint>
#include <cstddef>

namespace ms
{
	// What USE_SKILL carries besides the skill, as in the MapleRoyals client
	// (0x96d399, called from 0x969e21).
	namespace skill_use
	{
		constexpr int32_t SHADOW_STARS = 4121006;
		constexpr int32_t DISPEL = 2311001;
		constexpr int32_t RESURRECTION = 2321006;

		// Whether the packet carries where the user stands (0x96d6ca): the
		// skills that place something, or reach the party around them.
		inline bool sends_position(int32_t skill)
		{
			switch (skill)
			{
			case 1001003: case 1101006: case 1111007: case 1121000: case 1201006:
			case 1211009: case 1221000: case 1301006: case 1301007: case 1311007:
			case 1321000: case 2101001: case 2101003: case 2121000: case 2201001:
			case 2201003: case 2221000: case 2301004: case 2311001: case 2311003:
			case 2321000: case 2321005: case 3121000: case 3121002: case 3221000:
			case 4101004: case 4111001: case 4121000: case 4201003: case 4221000:
			case 5121000: case 5121009: case 5221000: case 11001001: case 11101003:
			case 12101000: case 12101001: case 14101003: case 15111005: case 21121000:
			case 22171000: case 22181000:
				return true;
			default:
				return false;
			}
		}

		// Whom a skill reaches besides the user (0x966f7a): the party
		// members around them, the mobs around them, or both.
		enum Reach : uint8_t
		{
			SELF = 0,
			PARTY = 2,
			MOBS = 4
		};

		inline uint8_t reach(int32_t skill)
		{
			switch (skill)
			{
			case 1101006: case 1121000: case 1221000: case 1301006: case 1301007:
			case 1321000: case 2101001: case 2121000: case 2201001: case 2221000:
			case 2301004: case 2311003: case 2321000: case 2321005: case RESURRECTION:
			case 3121000: case 3121002: case 3221000: case 3221002: case 4101004:
			case 4111001: case 4121000: case 4201003: case 4221000: case 5121000:
			case 5121009: case 5121010: case 5221000: case 11101003: case 12101000:
			case 14101003: case 15111005: case 21120007: case 21121000:
				return PARTY;
			case 1111007: case 1201006: case 1211009: case 1311007: case 2101003:
			case 2111004: case 2201003: case 2211004: case 2311005: case 4111003:
			case 4121004: case 4221004: case 12101001: case 12111002: case 14111001:
				return MOBS;
			case DISPEL:
				return PARTY | MOBS;
			default:
				return SELF;
			}
		}

		// The party a skill reaches, a bit for each of the six slots, the
		// first slot highest (0x96db3f). Without a party it is the user
		// alone.
		constexpr uint8_t ALONE = 0x80;

		inline uint8_t party_bit(size_t slot)
		{
			return static_cast<uint8_t>(1 << (5 - slot));
		}

		// Whether a party member counts: Resurrection reaches only the dead,
		// and never the user; the rest reach the living.
		inline bool reaches(int32_t skill, bool self, bool dead)
		{
			if (skill == RESURRECTION)
				return !self && dead;

			return self || !dead;
		}

		// Resurrection with no one to raise isn't sent at all.
		inline bool sends(int32_t skill, uint8_t party)
		{
			return skill != RESURRECTION || party != 0;
		}

		// How many mobs a skill reaches: its mob count, less the party
		// members it reaches.
		inline uint8_t mob_limit(uint8_t mobcount, int32_t members)
		{
			return mobcount > members ? static_cast<uint8_t>(mobcount - members) : 0;
		}

		// Shadow Stars spends a whole set of stars (0x949f5c): the first
		// stack of throwing stars that, with the stacks of the same star
		// after it, holds as many as the skill needs. 0 if none does.
		template <typename Slots>
		int32_t shadow_star(const Slots &slots, int32_t needed)
		{
			if (needed <= 0)
				return 0;

			for (auto first = slots.begin(); first != slots.end(); ++first)
			{
				int32_t star = first->first;

				if (star / 10000 != 207 || first->second <= 0)
					continue;

				int32_t count = 0;

				for (auto next = first; next != slots.end(); ++next)
					if (next->first == star)
						count += next->second;

				if (count >= needed)
					return star;
			}

			return 0;
		}
	}
}
