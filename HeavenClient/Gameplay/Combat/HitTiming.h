#pragma once

#include "Attack.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>

namespace ms
{
	namespace hit_timing
	{
		// Whether a close attack's hits land as it starts: Corkscrew Blow's do, for it carries
		// the player through the mobs (0x66b0ad).
		inline bool at_once(int32_t skill)
		{
			return skill == 5101004 || skill == 15101003;
		}

		// How long after an attack's first hit its `target`th mob (in target
		// order) is hit, as in the MapleRoyals client.
		inline uint16_t target_delay(Attack::Type type, int32_t skill, size_t target)
		{
			switch (type)
			{
			case Attack::Type::CLOSE:
				// Meso Explosion hits every mob at once (FUN_00980c90).
				if (skill == 4211006 || at_once(skill))
					return 0;

				// Band of Thieves's mob at once and the others together (0x9520a8).
				if (skill == 4211004)
					return target ? 280 : 0;

				// Each mob 70 ms after the last, up to 280 ms (FUN_00950921).
				return static_cast<uint16_t>(std::min<size_t>(target * 70, 280));
			case Attack::Type::MAGIC:
				// FUN_0095571f.
				switch (skill)
				{
				case 2201005: // Thunderbolt
				case 2301002: // Heal
				case 2111002: // Explosion
				case 2211002: // Ice Strike
				case 2311004: // Shining Ray
				case 2121001: // Big Bang
				case 2221001:
				case 2321001:
				case 4121004: // Ninja Ambush
				case 4221004:
				case 12101006: // Fire Pillar
					return static_cast<uint16_t>(std::min<size_t>(target * 50, UINT16_MAX));
				case 2221006: // Chain Lightning
					return static_cast<uint16_t>(std::min<size_t>(target * 100, UINT16_MAX));
				default:
					return 0;
				}
			default:
				return 0;
			}
		}

		// How long after a mob's first hit its `hit`th lands (0x66b05e): each of an attack's hits
		// comes after the last, Strafe's twice as fast. Barrage keeps its own times.
		inline uint16_t stagger(int32_t skill, size_t hit)
		{
			switch (skill)
			{
			case 5101004: // Corkscrew Blow
			case 15101003:
			case 5121007: // Barrage
			case 15111004:
				return 0;
			case 3111006: // Strafe
			case 3211006:
			case 13111001:
				return static_cast<uint16_t>(60 * hit);
			default:
				return static_cast<uint16_t>(120 * hit);
			}
		}

		// A close attack on one mob.
		struct CloseAttack
		{
			int32_t skill;
			// Whether the player made it; the attacks of others have fewer special cases.
			bool own;
			bool twohanded;
			// When the attack's first hit frame comes, and how long after that this mob's turn.
			uint16_t first;
			uint16_t target;
			int8_t speed;
			// How many hits it takes.
			size_t hits;
		};

		// 80 ms a hit, but `second` for the second, all added up (0x95264b).
		inline int32_t climb(size_t hit, int32_t second)
		{
			return static_cast<int32_t>(80 * (hit + 1)) + (hit > 0 ? second - 80 : 0);
		}

		// When a close attack's `hit`th hit on the mob lands, in ms from the attack's start
		// (0x95253c, and for others 0x9810d5).
		inline uint16_t close_hit(const CloseAttack &attack, size_t hit)
		{
			constexpr int32_t BARRAGE[] = { 450, 720, 990, 1260, 1710, 1980 };
			constexpr int32_t DEMOLITION[] = { 0, 180, 450, 720, 900, 990, 1080, 1170 };

			int32_t speed = attack.speed + 10;
			int32_t turn = attack.first + attack.target;
			int32_t time = turn;

			switch (attack.skill)
			{
			case 5101003: // Double Uppercut
				time = hit == 1 ? (attack.own ? 350 : 240) : 80;
				break;
			case 5121007: // Barrage
			case 15111004:
			{
				size_t step = hit / std::max<size_t>(attack.hits / 6, 1);
				time = BARRAGE[std::min<size_t>(step, 5)] * speed / 16;
				break;
			}
			case 5121004: // Demolition
				if (attack.own)
					time = attack.target + DEMOLITION[std::min<size_t>(hit, 7)] * speed / 16;

				break;
			case 5111006: // Shockwave
			case 15111003:
				if (attack.own)
					time = 1380 + attack.target;

				break;
			case 4221001: // Assassinate
				time += climb(hit, 240);
				break;
			case 4221007: // Boomerang Step
				if (attack.own)
					time += climb(hit, 240);

				break;
			case 1121008: // Brandish
			case 11111004:
				if (attack.own)
					time += climb(hit, attack.twohanded ? 240 : 160);

				break;
			}

			return static_cast<uint16_t>(time + stagger(attack.skill, hit));
		}
	}
}
