#include "../Character/MobDamage.h"
#include <cmath>
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
		using namespace ms::mob_damage;

		require(job_category(0) == 0 && job_category(232) == 2 && job_category(1110) == 1,
				"the job category is the hundreds digit, Cygnus included");

		require(standard_pdd(0, 1) == 7 && standard_pdd(0, 7) == 17 && standard_pdd(0, 30) == 19,
				"a level takes the last listed value at or below it");
		require(standard_pdd(1, 9) == 0, "levels before the first entry expect no W.DEF");
		require(standard_pdd(2, 30) == 75 && standard_pdd(4, 100) == 331 && standard_pdd(5, 200) == 309,
				"the table covers every job");

		// A new beginner in starter clothes.
		Defender beginner{ 1, 0, 12, 5, 4, 4, 7, 0, 4 };
		Attacker snail{ 1, 20 };
		require(stat_factor(beginner) == 7, "STR counts most for everyone but warriors");
		require(physical(beginner, snail, 12, 0.0, 0) == 1 && physical(beginner, snail, 12, 1.0, 0) == 1,
				"a snail always hits a beginner for 1");
		require(std::abs(physical_miss_chance(beginner, snail) - 400.0 / 90.0) < 1e-9,
				"the miss chance is avoid over 4.5 accuracy");

		// Stumpy (level 35, attack 125, accuracy 80).
		Attacker stumpy{ 35, 80 };

		// A level 30 warrior with exactly the expected W.DEF.
		Defender warrior{ 30, 110, 100, 30, 4, 4, 154, 0, 20 };
		require(stat_factor(warrior) == 37, "warriors weigh STR, DEX and LUK differently");
		require(physical(warrior, stumpy, 125, 0.0, 0) == 74, "the lowest Stumpy hit on a warrior");
		require(physical(warrior, stumpy, 125, 1.0, 0) == 82, "the highest Stumpy hit on a warrior");
		require(physical(warrior, stumpy, 125, 1.0, 50) == 41, "Invincible takes its share off last");

		require(shown(1000, 850, 0, 0) == 850, "the number leaves out Achilles' share");
		require(shown(101, 1000, 78, 1000) == 51, "and the half Meso Guard saves");
		require(shown(100, 1000, 78, 20) == 75, "or what the mesos buy");
		require(shown(-1, 850, 78, 1000) == -1 && shown(0, 850, 0, 0) == 0, "a dodge or miss stays");
		require(physical(warrior, stumpy, 125, 1.0, 50, 200) == 82, "then a Power Up scales what is left");

		// A level 30 magician short of the expected W.DEF takes more than
		// the raw attack.
		Defender magician{ 30, 210, 4, 25, 150, 4, 30, 100, 10 };
		require(physical(magician, stumpy, 125, 0.0, 0) == 137, "a W.DEF shortfall adds damage");

		// The same shortfall weighs less against a weaker monster.
		Attacker weaker{ 25, 80 };
		require(physical(magician, weaker, 125, 0.0, 0) == 127, "the shortfall scales with the level gap");

		// Monsters above the player's level take half the gap off their avoid.
		Defender thief{ 30, 410, 4, 60, 4, 100, 60, 0, 100 };
		require(std::abs(physical_miss_chance(thief, stumpy) - 98.0 / 360.0 * 100.0) < 1e-9,
				"a higher monster lowers the player's avoid");
		require(physical_miss_chance(magician, stumpy) == 8.0 / 360.0 * 100.0, "the general clamp starts at 2");
		require(physical_miss_chance(Defender{ 30, 0, 4, 4, 4, 4, 0, 0, 999 }, snail) == 80.0,
				"most players miss at most 80% of the time");
		require(physical_miss_chance(Defender{ 30, 400, 4, 4, 4, 4, 0, 0, 999 }, snail) == 95.0,
				"thieves can dodge up to 95%");
		require(physical_miss_chance(Defender{ 30, 400, 4, 4, 4, 4, 0, 0, 0 }, stumpy) == 5.0,
				"thieves dodge at least 5%");

		require(blinded(30, 0.29) && !blinded(30, 0.3) && !blinded(0, 0.0), "Blind misses by its percent");

		// Magic: STR, DEX, LUK and M.DEF soak it, more for magicians.
		require(magic(magician, 200, 0.0, 0) == 268, "magicians soak 30% of their magic defense");
		require(magic(warrior, 200, 0.0, 0) == 294, "everyone else soaks 25%");
		require(magic(magician, 200, 0.0, 50) == 134, "element resistance comes after");
		require(magic(magician, 200, 0.0, 0, 50) == 134, "a Magic Up scales the hit");
		require(magic(magician, 200, 0.0, 50, 200) == 268, "before element resistance");
		require(magic(magician, 10, 0.0, 0) == 1, "magic takes at least 1");

		Defender evasive{ 30, 210, 4, 25, 150, 4, 30, 100, 20 };
		require(magic_misses(evasive, Attacker{ 30, 1 }, 0.0), "accuracy under a tenth of avoid always misses");
		require(!magic_misses(evasive, Attacker{ 30, 21 }, 0.999), "accuracy over the avoid never misses");
		require(magic_misses(evasive, Attacker{ 30, 11 }, 0.5) && !magic_misses(evasive, Attacker{ 30, 12 }, 0.5),
				"the roll falls between a tenth of the avoid and all of it");

		// Power Guard at 30% against a mob of level 10, 30 avoid and 1000 HP,
		// for a level 10 player with 82 accuracy.
		Reflectee reflectee{ 10, 30, 1000, 0, false, false };
		Reflected reflected = power_guard(100, 30, reflectee, 10, 82);
		require(reflected.damage == 30 && reflected.shown == 30, "Power Guard sends back its percent");
		require(power_guard(-1, 30, reflectee, 10, 82).damage == 0, "a dodge sends nothing back");
		require(power_guard(100, 30, Reflectee{ 10, 30, 200, 0, false, false }, 10, 82).damage == 20,
				"at most a tenth of the mob's HP");
		require(power_guard(100, 30, Reflectee{ 10, 30, 1000, 0, true, false }, 10, 82).damage == 15,
				"half that for a boss");
		reflected = power_guard(100, 30, Reflectee{ 10, 30, 1000, 1, false, false }, 10, 82);
		require(reflected.damage == 1 && reflected.shown == 1, "a fixedDamage mob takes no more");
		reflected = power_guard(100, 30, Reflectee{ 10, 30, 1000, 50, false, false }, 10, 82);
		require(reflected.damage == 30 && reflected.shown == 50, "but shows its fixed damage");
		require(power_guard(100, 30, Reflectee{ 10, 30, 1000, 0, false, true }, 10, 82).shown == 0,
				"an invincible mob takes nothing");
		require(power_guard(100, 30, Reflectee{ 20, 30, 1000, 0, false, false }, 10, 82).shown == 0,
				"82 accuracy reaches 32 avoid at the player's level, 23 ten levels up");

		// Mana Reflection, with 100 INT and 4 LUK: 50.
		Reflectee caster{ 10, 16, 10000, 0, true, false };
		require(mana_reflection(100, 100, caster, 10, 100, 4) == 100, "Mana Reflection sends back its percent");
		require(mana_reflection(100, 100, Reflectee{ 10, 16, 1000, 0, true, false }, 10, 100, 4) == 50,
				"at most a twentieth of the mob's HP, bosses alike");
		require(mana_reflection(100, 100, Reflectee{ 10, 17, 10000, 0, false, false }, 10, 100, 4) == 0,
				"50 reaches 16 avoid at the player's level, not 17");
		require(mana_reflection(100, 100, Reflectee{ 11, 16, 10000, 0, false, false }, 10, 100, 4) == 100 &&
					mana_reflection(100, 100, Reflectee{ 12, 16, 10000, 0, false, false }, 10, 100, 4) == 0,
				"nor 16 two levels up");

		// Only fourth-job thieves shift and fourth-job knights guard.
		require(shadow_shifter(412) == 4120002 && shadow_shifter(422) == 4220002 && shadow_shifter(411) == 0,
				"Shadow Shifter is the Night Lord's and the Shadower's");
		require(guardian(112) == 1120005 && guardian(122) == 1220006 && guardian(132) == 0,
				"Guardian is the Hero's and the Paladin's");
		require(guard(true, true) == 2 && guard(true, false) == 1 && guard(false, true) == 0,
				"the Guardian byte tells a touch from an attack");

		std::cout << "Mob damage passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
