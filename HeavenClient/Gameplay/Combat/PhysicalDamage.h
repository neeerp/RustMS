#pragma once

#include "MagicDamage.h"

#include <cstdint>
#include <string>

namespace ms
{
	// How a blow, a shot or a throw hits a monster, as the MapleRoyals
	// client rolls it (CalcDamage::PDamage, FUN_0078df87), and the damage
	// range the stat window shows (FUN_008c2870). Each roll is a fraction
	// in [0, 1) that picks a value between two bounds.
	namespace physical_damage
	{
		using magic_damage::between;

		// The weapon types the client tells apart (FUN_00460aa0): an item's
		// id / 10000 % 100.
		enum Weapon : int32_t
		{
			NONE = 0,
			SWORD = 30,
			AXE = 31,
			BLUNT = 32,
			DAGGER = 33,
			WAND = 37,
			STAFF = 38,
			BAREHANDS = 39,
			SWORD_2H = 40,
			AXE_2H = 41,
			BLUNT_2H = 42,
			SPEAR = 43,
			POLEARM = 44,
			BOW = 45,
			CROSSBOW = 46,
			CLAW = 47,
			KNUCKLE = 48,
			GUN = 49
		};

		// The type of the weapon with this id. With none, the client fights
		// bare-handed when a check we haven't traced allows it; here it
		// always does.
		inline Weapon weapon(int32_t itemid)
		{
			if (itemid <= 0)
				return BAREHANDS;

			if (itemid / 1000000 != 1)
				return NONE;

			int32_t type = itemid / 10000 % 100;

			if ((type >= SWORD && type <= DAGGER) || (type >= WAND && type <= GUN))
				return static_cast<Weapon>(type);

			return NONE;
		}

		// Whether the attack is a shot (bow, crossbow, claw and gun skills
		// and regular attacks) or a blow; masteries and criticals depend on
		// it.
		enum Reach
		{
			CLOSE,
			SHOT
		};

		// The reach the stat window's mastery is for: a shot with a bow,
		// crossbow, claw or gun.
		inline Reach shown_reach(Weapon weapon)
		{
			switch (weapon)
			{
			case BOW:
			case CROSSBOW:
			case CLAW:
			case GUN:
				return SHOT;
			default:
				return CLOSE;
			}
		}

		// The reach whose mastery adds its accuracy to the character's
		// (SetFrom, FUN_0077f4c9): an archer's, an Assassin's or a
		// Gunslinger's. A Night Walker's claw adds none.
		inline Reach stat_reach(int32_t job)
		{
			return job % 1000 / 100 == 3 || job / 10 == 41 || job / 10 == 52 ? SHOT : CLOSE;
		}

		// A mastery skill as the character has it: its mastery and x, or
		// 0s if it isn't learned.
		struct Learned
		{
			int32_t mastery = 0;
			int32_t x = 0;
		};

		// A weapon's mastery, and what it adds to accuracy and attack.
		struct Mastery
		{
			int32_t mastery = 0;
			int32_t accuracy = 0;
			int32_t attack = 0;
		};

		// The mastery for a weapon, a reach and the skill used
		// (FUN_00764795). `learned` gives a Learned for a skill id. Each
		// weapon has its skills, and most only for one reach; a few skills
		// keep a mastery from the other.
		template<typename Lookup>
		Mastery mastery(Weapon weapon, Reach reach, int32_t job, int32_t skill, Lookup learned)
		{
			Mastery result;
			bool cygnus = job / 1000 == 1;

			auto base = [&](int32_t id)
			{
				Learned skill = learned(id);
				result.accuracy = skill.x;

				return skill.mastery;
			};

			auto expert = [&](int32_t id)
			{
				Learned skill = learned(id);
				result.attack = skill.x;

				return skill.mastery;
			};

			switch (weapon)
			{
			case SWORD:
			case SWORD_2H:
				if (reach == CLOSE || skill == 11101004)
				{
					for (int32_t id : { 1100000, 1200000, 11100000 })
						if ((result.mastery = base(id)))
							break;
				}
				break;
			case AXE:
			case AXE_2H:
				if (reach == CLOSE)
					result.mastery = base(1100001);
				break;
			case BLUNT:
			case BLUNT_2H:
				if (reach == CLOSE)
					result.mastery = base(1200001);
				break;
			case DAGGER:
				if (reach == CLOSE)
					result.mastery = base(4200000);
				break;
			case SPEAR:
				// Beholder adds its own.
				if (reach == CLOSE)
					result.mastery = base(1300000) + learned(1321007).mastery;
				break;
			case POLEARM:
				if (reach == CLOSE || skill == 21100004 || skill == 21110004 || skill == 21120006)
				{
					if (job / 100 == 21 || job == 2000)
					{
						// High Mastery replaces Polearm Mastery.
						int32_t mastery = base(21100000);
						int32_t high = expert(21120001);
						result.mastery = high ? high : mastery;
					} else
					{
						result.mastery = base(1300001) + learned(1321007).mastery;
					}
				}
				break;
			case BOW:
				// Bow Expert replaces Bow Mastery but keeps its accuracy.
				if (reach == SHOT)
				{
					int32_t mastery = base(cygnus ? 13100000 : 3100000);
					int32_t bowexpert = expert(cygnus ? 13110003 : 3120005);
					result.mastery = bowexpert ? bowexpert : mastery;
				}
				break;
			case CROSSBOW:
				if (reach == SHOT)
				{
					int32_t mastery = base(3200000);
					int32_t boost = expert(3220004);
					result.mastery = boost ? boost : mastery;
				}
				break;
			case CLAW:
				if (reach == SHOT || skill == 14111006)
					result.mastery = base(cygnus ? 14100000 : 4100000);
				break;
			case KNUCKLE:
				if (reach == CLOSE || skill == 5121002 || skill == 15111007)
					result.mastery = base(cygnus ? 15100001 : 5100001);
				break;
			case GUN:
				if (reach == SHOT || skill == 5221003)
					result.mastery = base(5200000);
				break;
			default:
				break;
			}

			return result;
		}

		// Where the stat roll starts: (mastery * 5 + 10) * 0.9% of the stat,
		// so 9% with no mastery.
		inline double least(int32_t mastery)
		{
			return (mastery * 5 + 10) * 0.009;
		}

		// What the character's body does, as far as the damage cares.
		enum Motion
		{
			OTHER,
			// swingO1 to swingPF (actions 5 to 15).
			SWING,
			// proneStab (0x20).
			PRONE,
			// A pirate's somersault or straight (0x4e, 0x4f).
			KICK,
			// fireburner or coolingeffect (0x5f, 0x60).
			BURN,
			// Any action that throws or fires (FUN_004566f5).
			SHOOT
		};

		// The motion of an action or stance. Some throws use the swing
		// names, so a shot is a shot whatever it is called.
		inline Motion motion(const std::string &action, bool shoots)
		{
			if (shoots)
				return SHOOT;

			if (action == "proneStab")
				return PRONE;

			if (action == "somersault" || action == "straight")
				return KICK;

			if (action == "fireburner" || action == "coolingeffect")
				return BURN;

			for (const char *swing : { "swingO1", "swingO2", "swingO3", "swingOF", "swingT1", "swingT2", "swingT3",
									   "swingTF", "swingP1", "swingP2", "swingPF" })
				if (action == swing)
					return SWING;

			return OTHER;
		}

		// Whether a skill counts as a swing whatever the body does: Rush
		// and Blast three times in five, Brandish's second hit, and every
		// Aran blow (0x78e9b2..0x78ea1b).
		inline bool forced_swing(int32_t skill, int32_t job, size_t hit, double roll)
		{
			switch (skill)
			{
			case 1121006:
			case 1221007:
			case 1321003:
			case 1221009:
				return between(0.0, 5.0, roll) < 3.0;
			case 1121008:
			case 11111004:
				if (hit == 1)
					return true;
				break;
			default:
				break;
			}

			return job / 100 == 21 && job % 1000 / 100 == 1;
		}

		// The total STR, DEX and LUK.
		struct Stats
		{
			int32_t str = 0;
			int32_t dex = 0;
			int32_t luk = 0;
		};

		// How a hit comes from its stats: a stat rolled between `least` of
		// it and all of it, times the multiplier, plus the other stats,
		// times PAD over the divisor.
		struct Formula
		{
			int32_t primary = 0;
			double least = 0.0;
			double multiplier = 0.0;
			int32_t secondary = 0;
			double divisor = 100.0;
		};

		// The hit from a formula, PAD and a roll.
		inline double raw(const Formula &formula, int32_t pad, double roll)
		{
			double rolled = between(formula.primary * formula.least, formula.primary, roll);

			return (rolled * formula.multiplier + formula.secondary) * pad / formula.divisor;
		}

		// The lowest and highest raw hits.
		inline double lowest(const Formula &formula, int32_t pad)
		{
			return raw(formula, pad, 0.0);
		}

		inline double highest(const Formula &formula, int32_t pad)
		{
			return (formula.primary * formula.multiplier + formula.secondary) * pad / formula.divisor;
		}

		inline bool thief(int32_t job)
		{
			return job % 1000 / 100 == 4;
		}

		// The formula for a weapon, wielded or swung (FUN_0078df87's switch
		// at 0x78ef9c). A dagger is a thief's only.
		inline Formula wielded(Weapon weapon, int32_t job, int32_t skill, bool swing, Stats stats, double least)
		{
			auto strong = [&](double multiplier) { return Formula{ stats.str, least, multiplier, stats.dex }; };
			auto nimble = [&](double multiplier) { return Formula{ stats.dex, least, multiplier, stats.str }; };
			auto lucky = [&](double multiplier) { return Formula{ stats.luk, least, multiplier, stats.str + stats.dex }; };

			switch (weapon)
			{
			case SWORD:
				return strong(4.0);
			case DAGGER:
				return thief(job) ? lucky(3.6) : strong(4.0);
			case AXE:
			case BLUNT:
			case WAND:
			case STAFF:
				return strong(swing ? 4.4 : 3.2);
			case BAREHANDS:
				return strong(job == 500 ? 3.0 : 4.2);
			case SWORD_2H:
				return strong(4.6);
			case AXE_2H:
			case BLUNT_2H:
				return strong(swing ? 4.8 : 3.4);
			case SPEAR:
			case POLEARM:
				// A spear stabs best and a polearm swings best; Dragon Roar
				// is neither.
				if (skill == 1311006)
					return strong(4.0);

				return strong((weapon == SPEAR) == swing ? 3.0 : 5.0);
			case BOW:
				return nimble(3.4);
			case CROSSBOW:
			case GUN:
				return nimble(3.6);
			case CLAW:
				// Lucky Seven and Triple Throw roll from half the LUK.
				switch (skill)
				{
				case 4001344:
				case 4121007:
				case 14001004:
				case 14111005:
					return Formula{ stats.luk, 0.5, 5.0, 0 };
				default:
					return lucky(3.6);
				}
			case KNUCKLE:
				return strong(4.8);
			default:
				return Formula{};
			}
		}

		// The formula for a hit: a whack with a bow, crossbow, claw or gun
		// that throws or fires nothing has its own, then a stab from prone,
		// then the weapon's.
		inline Formula formula(Weapon weapon, int32_t job, int32_t skill, Motion motion, bool swing, Stats stats,
							   double least)
		{
			bool shot = motion == SHOOT;

			switch (weapon)
			{
			case BOW:
			case CROSSBOW:
				if (!shot)
					return Formula{ stats.dex, least, 3.4, stats.str, 150.0 };
				break;
			case GUN:
				if (!shot && motion != BURN && skill != 5201002 && skill != 5221003 && skill != 5221007 &&
					skill != 5221008)
				{
					if (motion == KICK)
						return Formula{ stats.dex, least, 1.8, stats.str };

					return Formula{ stats.dex, least, 1.0, stats.str, 200.0 };
				}
				break;
			case CLAW:
				if (!shot && skill != 4221001 && skill != 4121008 && skill != 14101006 && skill != 14111006)
					return Formula{ stats.luk, least, 1.0, stats.str + stats.dex, 150.0 };
				break;
			default:
				break;
			}

			if (motion == PRONE)
				return Formula{ stats.str, least, 1.0, stats.dex, 200.0 };

			return wielded(weapon, job, skill, swing, stats, least);
		}

		// Flamethrower and Ice Splitter do half without their capsules.
		inline double capsule(int32_t skill, int32_t bullet)
		{
			if (skill == 5211004 && bullet / 1000 != 2331)
				return 0.5;

			if (skill == 5211005 && bullet / 1000 != 2332)
				return 0.5;

			return 1.0;
		}

		// Whether an item is ammo whose PAD a shot adds: arrows, bolts,
		// stars or bullets, but not a Gunslinger's capsules.
		inline bool ammo(int32_t itemid)
		{
			int32_t type = itemid / 10000;

			if (itemid / 1000 == 2331 || itemid / 1000 == 2332)
				return false;

			return type == 206 || type == 207 || type == 233;
		}

		// The physical attack, at most 1999.
		inline int32_t attack(int32_t pad)
		{
			if (pad < 0)
				return 0;

			return pad < 1999 ? pad : 1999;
		}

		// The accuracy against a monster: lower for each level it is above
		// the player.
		inline double accuracy(int32_t acc, int32_t playerlevel, int32_t moblevel)
		{
			if (acc < 0)
				acc = 0;

			if (acc > 999)
				acc = 999;

			int32_t above = moblevel > playerlevel ? moblevel - playerlevel : 0;

			return acc * 100.0 / (above * 10.0 + 255.0);
		}

		// Whether a blow lands: a roll between 70% and 130% of the accuracy
		// must reach the monster's avoid, which counts between 0 and 999.
		inline bool hits(double accuracy, int32_t avoid, double roll)
		{
			if (avoid < 0)
				avoid = 0;

			if (avoid > 999)
				avoid = 999;

			return between(accuracy * 0.7, accuracy * 1.3, roll) >= avoid;
		}

		// Darkness makes four blows in five miss.
		inline bool sees(double roll)
		{
			return between(0.0, 100.0, roll) <= 20.0;
		}

		// A hit of an element on a monster with an attribute, with the
		// element counting `amplify` times (FUN_00790782).
		inline double attribute(double damage, uint8_t attribute, double amplify)
		{
			switch (attribute)
			{
			case magic_damage::IMMUNE:
				return (1.0 - amplify) * damage;
			case magic_damage::STRONG:
				return (1.0 - amplify * 0.5) * damage;
			case magic_damage::WEAK:
			{
				double weak = (amplify * 0.5 + 1.0) * damage;

				if (weak <= damage)
					return damage;

				return weak < 199999.0 ? weak : 199999.0;
			}
			default:
				return damage;
			}
		}

		// A White Knight's, Dawn Warrior's, Thunder Breaker's or Aran's
		// charge: its element, damage and how much the element counts.
		struct Charge
		{
			uint8_t element = 0;
			int32_t damage = 100;
			int32_t amplify = 0;
		};

		// The element of a charge skill (FUN_007908e7), or 0.
		inline uint8_t charge_element(int32_t skill)
		{
			switch (skill)
			{
			case 1211003:
			case 1211004:
				return 2;
			case 1211005:
			case 1211006:
			case 21111005:
				return 1;
			case 1211007:
			case 1211008:
			case 15101006:
				return 3;
			case 1221003:
			case 1221004:
			case 11111007:
				return 5;
			default:
				return 0;
			}
		}

		// A hit with a charge on (FUN_007907ec): its damage percent, and
		// its element against the monster's attributes.
		inline double charged(double damage, const Charge &charge, const magic_damage::Attributes &attributes)
		{
			if (!charge.element)
				return damage;

			return attribute(charge.damage * 0.01 * damage, attributes[charge.element % magic_damage::ELEMENTS],
							 charge.amplify * 0.01);
		}

		// Whether the monster's defense counts: Sacrifice, Shadow Meso,
		// Assaulter and Demolition go through it.
		inline bool defended(int32_t skill)
		{
			return skill != 1311005 && skill != 4111004 && skill != 4211002 && skill != 5121004;
		}

		// The monster's weapon defense takes 50% to 60% of itself off the
		// hit; it counts at most 1999.
		inline double defend(double damage, int32_t pdef, double roll)
		{
			return magic_damage::defend(damage, pdef, roll);
		}

		// The weapon's critical skill (FUN_00765095): Critical Shot for a
		// bow's or crossbow's shot, Critical Throw for a claw's, and
		// Critical Punch for any knuckle blow.
		inline int32_t critical_skill(Weapon weapon, Reach reach, int32_t job)
		{
			bool cygnus = job / 1000 == 1;

			switch (weapon)
			{
			case BOW:
			case CROSSBOW:
				return reach == SHOT ? (cygnus ? 13000000 : 3000001) : 0;
			case CLAW:
				return reach == SHOT ? (cygnus ? 14100001 : 4100001) : 0;
			case KNUCKLE:
				return 15110000;
			default:
				return 0;
			}
		}

		// A critical's rate and damage, both in percent: a damage of 150
		// adds half the hit.
		struct Critical
		{
			int32_t rate = 0;
			int32_t damage = 0;
			bool source = false;
		};

		inline int32_t percent_cap(int32_t percent)
		{
			if (percent < 0)
				return 0;

			return percent < 100 ? percent : 100;
		}

		// Sharp Eyes' value adds its high byte to the rate and its low byte
		// to the damage.
		inline Critical sharpen(Critical critical, int32_t sharpeyes)
		{
			if (sharpeyes <= 0)
				return critical;

			critical.rate += percent_cap(sharpeyes >> 8);
			critical.damage = critical.damage ? critical.damage + (sharpeyes & 0xFF) : 100 + (sharpeyes & 0xFF);
			critical.source = true;

			return critical;
		}

		constexpr int32_t STUN_MASTERY = 5110000;

		// Stun Mastery's on a stunned monster: its rate and its whole
		// damage add on.
		inline Critical stun(Critical critical, int32_t rate, int32_t damage)
		{
			critical.rate += percent_cap(rate);
			critical.damage += damage;
			critical.source = true;

			return critical;
		}

		// Whether a hit is critical. Blizzard, Shadow Meso and Assassinate's
		// first strike have their own; Assassinate's second takes its rate and
		// damage from the skill (0x790100).
		inline bool critical(const Critical &critical, int32_t skill, double roll, bool second_strike = false)
		{
			if (skill == 3211003 || skill == 4111004 || (skill == 4221001 && !second_strike))
				return false;

			if (critical.rate <= 0 || critical.damage <= 0 || !critical.source)
				return false;

			return between(0.0, 100.0, roll) < critical.rate;
		}

		// A Barrage's fifth hit does twice, and its sixth four times.
		inline double barrage(double damage, int32_t skill, size_t hit)
		{
			if (skill != 5121007 && skill != 15111004)
				return damage;

			if (hit == 4)
				return damage * 2.0;

			if (hit == 5)
				return damage * 4.0;

			return damage;
		}

		// Everything a blow's damage comes from that the character knows
		// before it meets a monster.
		struct Blow
		{
			Weapon weapon = NONE;
			int32_t job = 0;
			Stats stats;
			// The physical attack without ammo, and what the ammo a shot
			// throws adds.
			int32_t attack = 0;
			int32_t ammo = 0;
			int32_t mastery = 0;
			Motion motion = OTHER;
			// The skill's share of the hit, and the combo's percent or 0.
			double scale = 1.0;
			int32_t combo = 0;
			// How much the skill's element counts, in percent: Inferno's and
			// Blizzard's x.
			int32_t amplify = 100;
			Critical critical;
			// Stun Mastery's rate and damage on a stunned monster.
			Critical stunned;
			Charge charge;
			// Berserk's percent, or 0.
			int32_t berserk = 0;
			// What Shadow Partner's hits do, in percent of the first set's.
			int32_t partner = 0;
			// Whether the player is in Darkness.
			bool blind = false;
			// What Assassinate multiplies each line by, from how long the
			// player was in Dark Sight; 1 for anything else.
			int32_t stealth = 1;
		};

		// A hit from its raw value, before the monster's guards, with a
		// critical adding to it. `base` is the raw hit after defense,
		// rounded down.
		inline double scaled(double damage, int32_t base, double scale, int32_t combo, bool crit, int32_t critdamage)
		{
			damage *= scale;

			if (combo > 0)
				damage = combo * 0.01 * damage;

			if (crit)
				damage += (critdamage - 100) * 0.01 * base;

			return damage;
		}

		// Power Explosion's percent of a hit, after all else (0x79053b); 0 when it is off.
		inline double exploded(double damage, int32_t explosion)
		{
			return explosion ? damage * explosion * 0.01 : damage;
		}

		// The number shown: between 1 and 199999, rounded down.
		inline int32_t finish(double damage)
		{
			return magic_damage::finish(damage);
		}
	}
}
