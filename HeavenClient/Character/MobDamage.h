#pragma once

#include "SkillId.h"

#include <algorithm>
#include <cstdint>
#include <utility>

namespace ms
{
	// How much a monster's body or attack takes from the player, as the v83
	// client works it out (FUN_0079309f and FUN_0079345e, with the misses in
	// FUN_0079286e and FUN_007929ca). The random rolls come in as arguments.
	namespace mob_damage
	{
		// The player's side of a hit.
		struct Defender
		{
			int32_t level;
			int32_t job;
			int32_t str;
			int32_t dex;
			int32_t intl;
			int32_t luk;
			int32_t wdef;
			int32_t mdef;
			int32_t avoid;
		};

		// The monster's side of a hit.
		struct Attacker
		{
			int32_t level;
			int32_t accuracy;
		};

		// 0 beginner, 1 warrior, 2 magician, 3 bowman, 4 thief, 5 pirate.
		inline int32_t job_category(int32_t job)
		{
			return job % 1000 / 100;
		}

		// The skill whose x percent a job resists a magic attack of
		// `element` (1 ice, 2 fire, 3 lightning, 4 poison) by, or 0
		// (FUN_00765a34). Dragon Knights, Priests and Blaze Wizards resist
		// every element, F/P Wizards fire and poison, I/L Wizards ice and
		// lightning.
		inline int32_t resistance_skill(int32_t job, int32_t element)
		{
			if (element < 1 || element > 7)
				return 0;

			auto line = [job](int32_t base) {
				return job / 10 == base / 10 && base % 10 <= job % 10;
			};

			if (line(131))
				return 1310000;

			if (line(211))
				return element == 2 || element == 4 ? 2110000 : 0;

			if (line(221))
				return element == 1 || element == 3 ? 2210000 : 0;

			if (line(231))
				return 2310000;

			if (line(1211))
				return 12110000;

			return 0;
		}

		// A job's Power Stance: the Hero's, the Paladin's, the Dark
		// Knight's or the Aran's, or 0 (0x958aa9).
		inline int32_t stance_skill(int32_t job)
		{
			switch (job)
			{
				case 112:
					return 1121002;
				case 122:
					return 1221002;
				case 132:
					return 1321002;
				case 2112:
					return 21121003;
				default:
					return 0;
			}
		}

		// The W.DEF a player of a job and level is expected to have
		// (Base.wz StandardPDD.img, read by FUN_00792d21). Each level takes
		// the last listed value at or below it, or 0 before the first.
		inline int32_t standard_pdd(int32_t category, int32_t level)
		{
			using Entry = std::pair<int16_t, int16_t>;

			static constexpr Entry beginner[] = { { 1, 7 }, { 5, 17 }, { 8, 19 } };
			static constexpr Entry warrior[] = {
				{ 10, 54 }, { 12, 57 }, { 15, 83 }, { 20, 106 }, { 22, 109 }, { 25, 129 }, { 30, 154 },
				{ 35, 179 }, { 40, 203 }, { 47, 208 }, { 50, 261 }, { 55, 267 }, { 60, 305 }, { 65, 308 },
				{ 70, 359 }, { 75, 356 }, { 80, 382 }, { 85, 388 }, { 90, 440 }, { 95, 446 }, { 100, 494 }
			};
			static constexpr Entry magician[] = {
				{ 8, 25 }, { 10, 31 }, { 13, 40 }, { 15, 49 }, { 18, 54 }, { 20, 56 }, { 25, 60 }, { 28, 64 },
				{ 30, 75 }, { 33, 91 }, { 35, 98 }, { 40, 99 }, { 48, 107 }, { 50, 131 }, { 55, 134 },
				{ 58, 142 }, { 60, 159 }, { 65, 162 }, { 68, 170 }, { 70, 184 }, { 75, 190 }, { 78, 198 },
				{ 80, 212 }, { 85, 218 }, { 88, 226 }, { 90, 240 }, { 95, 246 }, { 98, 254 }, { 100, 266 }
			};
			static constexpr Entry bowman[] = {
				{ 10, 32 }, { 15, 49 }, { 20, 65 }, { 25, 80 }, { 30, 95 }, { 35, 110 }, { 40, 125 },
				{ 50, 145 }, { 55, 148 }, { 60, 177 }, { 65, 180 }, { 70, 206 }, { 75, 212 }, { 80, 238 },
				{ 85, 244 }, { 90, 270 }, { 95, 276 }, { 100, 298 }
			};
			static constexpr Entry thief[] = {
				{ 10, 42 }, { 15, 60 }, { 20, 76 }, { 22, 85 }, { 25, 100 }, { 30, 115 }, { 32, 116 },
				{ 35, 131 }, { 37, 132 }, { 40, 147 }, { 50, 184 }, { 55, 187 }, { 60, 220 }, { 65, 223 },
				{ 70, 257 }, { 75, 263 }, { 80, 291 }, { 85, 297 }, { 90, 325 }, { 95, 331 }, { 100, 331 }
			};
			static constexpr Entry pirate[] = {
				{ 10, 24 }, { 15, 43 }, { 20, 56 }, { 25, 69 }, { 30, 82 }, { 35, 95 }, { 40, 108 },
				{ 50, 146 }, { 55, 149 }, { 60, 178 }, { 65, 181 }, { 70, 207 }, { 75, 213 }, { 80, 239 },
				{ 85, 245 }, { 90, 271 }, { 100, 309 }
			};

			auto lookup = [level](const auto &table) {
				int32_t value = 0;

				for (const Entry &entry : table)
					if (entry.first <= level)
						value = entry.second;

				return value;
			};

			switch (category)
			{
			case 0:
				return lookup(beginner);
			case 1:
				return lookup(warrior);
			case 2:
				return lookup(magician);
			case 3:
				return lookup(bowman);
			case 4:
				return lookup(thief);
			case 5:
				return lookup(pirate);
			default:
				return 0;
			}
		}

		// How much the player's stats add to their W.DEF.
		inline int32_t stat_factor(const Defender &player)
		{
			if (job_category(player.job) == 1)
				return static_cast<int32_t>(player.str * 0.2857142857142857 + player.dex * 0.25 +
											player.luk * 0.25 + player.intl * 0.1111111111111111);

			return static_cast<int32_t>(player.str * 0.4 + player.dex * 0.2857142857142857 +
										player.intl * 0.1111111111111111 + player.luk * 0.25);
		}

		// The player's avoid against a monster: those above the player's
		// level lose half the difference.
		inline int32_t avoid(const Defender &player, const Attacker &mob)
		{
			int32_t avoid = std::clamp(player.avoid, 0, 999);

			if (player.level < mob.level)
				avoid = std::max(0, avoid - (mob.level - player.level) / 2);

			return avoid;
		}

		// The percent chance that a physical hit misses.
		inline double physical_miss_chance(const Defender &player, const Attacker &mob)
		{
			int32_t accuracy = std::clamp(mob.accuracy, 0, 999);
			double chance = avoid(player, mob) / (accuracy * 4.5) * 100.0;

			if (job_category(player.job) == 4)
				return std::clamp(chance, 5.0, 95.0);

			return std::clamp(chance, 2.0, 80.0);
		}

		// A blinded monster's hit, touch or attack, misses by the Blind's
		// percent before any avoid roll (FUN_009581a9, 0x9585c3). `roll` is
		// uniform in [0, 1).
		inline bool blinded(int32_t blind, double roll)
		{
			return roll * 100.0 < blind;
		}

		// A magic hit misses when the monster's accuracy doesn't beat a roll
		// between a tenth of the player's avoid and all of it. `roll` is
		// uniform in [0, 1).
		inline bool magic_misses(const Defender &player, const Attacker &mob, double roll)
		{
			int32_t total = avoid(player, mob);
			double lowest = total * 0.1;

			return std::clamp(mob.accuracy, 0, 999) <= lowest + (total - lowest) * roll;
		}

		// A hit never takes less than 1 or more than 199999.
		inline int32_t finish(double damage)
		{
			return static_cast<int32_t>(std::clamp(damage, 1.0, 199999.0));
		}

		// A physical hit, for a monster with `attack` and a `roll` uniform in
		// [0, 1). `reduce` is the percent Invincible takes off (0x7933b2),
		// and a Power Up puts the rest at `percent` (0 for none).
		inline int32_t physical(const Defender &player, const Attacker &mob, int32_t attack, double roll,
								int32_t reduce, int32_t percent = 0)
		{
			attack = std::clamp(attack, 0, 1999);

			double base = (attack * 0.8 + attack * 0.05 * roll) * attack * 0.01;
			int32_t factor = stat_factor(player);
			int32_t standard = standard_pdd(job_category(player.job), player.level);
			int32_t wdef = player.wdef;
			double soak = (factor * 0.00125 + 0.28) * wdef;

			if (wdef >= standard)
			{
				soak += (wdef - standard) *
						(factor * 0.0011111111111111111 + player.level * 0.0007692307692307692 + 0.28) * 0.7;
			} else
			{
				double shortfall =
					(wdef - standard) * (player.level * 0.0018181818 + factor * 0.00125 + 0.28);

				if (mob.level <= player.level)
					soak += shortfall * 13.0 / (player.level - mob.level + 13);
				else
					soak += shortfall * 1.3;
			}

			base -= soak;
			base -= reduce * base * 0.01;

			if (percent != 0)
				base = base * percent * 0.01;

			return finish(base);
		}

		// A magic hit, for a monster with `attack` and a `roll` uniform in
		// [0, 1). `resist` is the percent the player resists of its element,
		// after a Magic Up puts it at `percent` (0 for none).
		inline int32_t magic(const Defender &player, int32_t attack, double roll, int32_t resist,
							 int32_t percent = 0)
		{
			attack = std::clamp(attack, 0, 1999);

			double base = (attack * 0.75 + attack * 0.05 * roll) * attack * 0.01;
			double weight = job_category(player.job) == 2 ? 0.3 : 0.25;

			base -= (player.str / 7.0 + player.dex / 6.0 + player.luk / 5.0 + player.mdef) * weight;

			if (percent != 0)
				base = base * percent * 0.01;

			int32_t damage = finish(base);
			damage = (100 - resist) * damage / 100;

			return std::max(damage, 1);
		}

		// The number shown over the player for a hit of `damage` (0x9593b8).
		// The client leaves Achilles, Meso Guard and Magic Guard to the
		// server, but the number leaves out what the first two save:
		// Achilles keeps `achilles` per mille off (1000 without it), and Meso
		// Guard saves half at `mesoguard` mesos per 100 (0 without it), or
		// what `meso` buys. Magic Guard's share still shows.
		inline int32_t shown(int32_t damage, int32_t achilles, int32_t mesoguard, int64_t meso)
		{
			if (damage <= 0)
				return damage;

			int32_t kept_off = (1000 - achilles) * damage / 1000;
			int32_t saved = 0;

			if (mesoguard > 0)
			{
				saved = damage / 2;

				if (meso < static_cast<int64_t>(saved) * mesoguard / 100)
					saved = static_cast<int32_t>(meso * 100 / mesoguard);
			}

			return damage - kept_off - saved;
		}

		// The MP Magic Guard takes of a hit of `damage` at its `percent`
		// (0 without it), no more than the `mp` left unless under Infinity
		// (0x959595). The client only foresees it, for the pet's potions.
		inline int32_t magic_guarded(int32_t damage, int32_t percent, int32_t mp, bool infinity)
		{
			if (damage <= 0 || percent <= 0)
				return 0;

			int32_t share = percent * damage / 100;

			return infinity ? share : std::min(share, mp);
		}

		// The Night Lord's or Shadower's Shadow Shifter, which may dodge any
		// hit, a miss too, at its chance (0x9584f6); 0 for other jobs.
		inline int32_t shadow_shifter(int32_t job)
		{
			switch (job)
			{
			case 412: return SkillId::SHADOW_SHIFTER_NIGHT_LORD;
			case 422: return SkillId::SHADOW_SHIFTER_SHADOWER;
			default: return 0;
			}
		}

		// The Hero's or Paladin's Guardian, which may block a hit that lands
		// while they hold a shield, at its chance per mille (0x958692); 0 for
		// other jobs.
		inline int32_t guardian(int32_t job)
		{
			switch (job)
			{
			case 112: return SkillId::GUARDIAN_HERO;
			case 122: return SkillId::GUARDIAN_PALADIN;
			default: return 0;
			}
		}

		// The Guardian byte of TAKE_DAMAGE: 2 for a touch it blocked, 1 for
		// an attack, 0 if it blocked nothing (0x9594d0).
		inline uint8_t guard(bool blocked, bool touch)
		{
			return blocked ? (touch ? 2 : 1) : 0;
		}

		// The mob that Power Guard or Mana Reflection sends some of its hit
		// back at.
		struct Reflectee
		{
			int32_t level;
			int32_t avoid;
			int32_t maxhp;
			int32_t fixeddamage;
			bool boss;
			bool invincible;
		};

		// What Power Guard sends back: what the mob takes and the player is
		// spared, and the number shown over the mob (0 a miss).
		struct Reflected
		{
			int32_t damage;
			int32_t shown;
		};

		// What Power Guard at `percent` sends back at a mob that touched a
		// player of `level` and `accuracy` for `damage` (FUN_009581a9 at
		// 0x9587c6): at most a tenth of the mob's HP, halved for a boss. A
		// fixedDamage mob shows its fixed damage and takes no more. Nothing
		// goes back at an invincible mob, or at one whose avoid beats the
		// accuracy, weighed down for each level the mob has on the player
		// (0x9588cd).
		inline Reflected power_guard(int32_t damage, int32_t percent, const Reflectee &mob, int32_t level,
									 int32_t accuracy)
		{
			if (damage <= 0 || percent <= 0 || mob.invincible)
				return { 0, 0 };

			int32_t reflected = std::min(damage * percent / 100, mob.maxhp / 10);

			if (mob.boss)
				reflected /= 2;

			int32_t shown = reflected;

			if (mob.fixeddamage > 0)
			{
				shown = mob.fixeddamage;
				reflected = std::min(reflected, mob.fixeddamage);
			}

			int32_t above = std::max(0, mob.level - level);
			double reach = std::clamp(accuracy, 0, 999) * 100.0 / (above * 10.0 + 255.0);

			if (std::clamp(mob.avoid, 0, 999) > reach)
				return { 0, 0 };

			return { reflected, shown };
		}

		// What Mana Reflection, its chance come up, shows sending `percent` of
		// a spell's `damage` back at the mob (0x958d50): at most a twentieth
		// of its HP, and nothing (a miss) if its avoid beats 5 for each 10 of
		// the player's INT and of their LUK, weighed down for each level the
		// mob has on them and by 0.85. Unlike Power Guard's, it spares the
		// player nothing.
		inline int32_t mana_reflection(int32_t damage, int32_t percent, const Reflectee &mob, int32_t level,
									   int32_t intelligence, int32_t luck)
		{
			if (damage <= 0 || percent <= 0)
				return 0;

			int32_t reflected = std::min(percent * damage / 100, mob.maxhp / 20);
			int32_t above = std::max(0, mob.level - level);
			double reach = (intelligence / 10 + luck / 10) * 5 * 100.0 / (above * 10.0 + 255.0) * 0.85;

			if (std::clamp(mob.avoid, 0, 999) > reach)
				return 0;

			return reflected;
		}

		// An attack with info/knockback throws the player flat, at 1900 pixels per second from a foothold and 900
		// from the air, a ladder or a rope, away from the mob: left when the mob is further right, else right
		// (0x958b0d). The sideways speed; the throw has none upwards.
		inline double thrown(bool grounded, int32_t mob_x, int32_t player_x)
		{
			double power = grounded ? 1900.0 : 900.0;

			return mob_x > player_x ? -power : power;
		}
	}
}
