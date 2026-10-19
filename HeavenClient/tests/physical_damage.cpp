#include "../Gameplay/Combat/PhysicalDamage.h"
#include <cmath>
#include <iostream>
#include <map>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool near(double a, double b)
{
	return std::fabs(a - b) < 1e-6;
}

int main()
{
	try
	{
		using namespace ms::physical_damage;

		require(weapon(1302000) == SWORD && weapon(1472030) == CLAW && weapon(1492000) == GUN, "a weapon's type");
		require(weapon(0) == BAREHANDS && weapon(1342000) == NONE && weapon(2070000) == NONE, "none, or not one");

		require(shown_reach(BOW) == SHOT && shown_reach(GUN) == SHOT && shown_reach(KNUCKLE) == CLOSE, "the window's reach");
		require(stat_reach(310) == SHOT && stat_reach(412) == SHOT && stat_reach(522) == SHOT, "archers, Assassins and Gunslingers shoot");
		require(stat_reach(422) == CLOSE && stat_reach(1410) == CLOSE && stat_reach(512) == CLOSE, "the rest don't");

		std::map<int32_t, Learned> book = {
			{ 1100000, { 10, 20 } },
			{ 3100000, { 10, 20 } },
			{ 3120005, { 16, 10 } },
			{ 1300000, { 10, 20 } },
			{ 1321007, { 5, 0 } },
			{ 21100000, { 10, 20 } },
			{ 21120001, { 15, 30 } }
		};
		auto learned = [&](int32_t id)
		{
			auto found = book.find(id);

			return found == book.end() ? Learned{} : found->second;
		};

		Mastery sword = mastery(SWORD, CLOSE, 110, 0, learned);
		require(sword.mastery == 10 && sword.accuracy == 20 && sword.attack == 0, "Sword Mastery");
		require(mastery(SWORD, SHOT, 110, 0, learned).mastery == 0, "not for a shot");
		require(mastery(SWORD, SHOT, 110, 11101004, learned).mastery == 10, "but for Soul Blade");

		Mastery bow = mastery(BOW, SHOT, 312, 0, learned);
		require(bow.mastery == 16 && bow.accuracy == 20 && bow.attack == 10, "Bow Expert replaces Bow Mastery");
		require(mastery(BOW, CLOSE, 312, 0, learned).mastery == 0, "not for a whack");
		book.erase(3120005);
		require(mastery(BOW, SHOT, 312, 0, learned).mastery == 10, "Bow Mastery alone");

		require(mastery(SPEAR, CLOSE, 132, 0, learned).mastery == 15, "Beholder adds its own");
		Mastery aran = mastery(POLEARM, CLOSE, 2112, 0, learned);
		require(aran.mastery == 15 && aran.accuracy == 20 && aran.attack == 30, "High Mastery replaces Polearm Mastery");
		require(mastery(POLEARM, SHOT, 2112, 21110004, learned).mastery == 15, "and keeps its own for Combo Smash");
		require(mastery(GUN, SHOT, 520, 0, learned).mastery == 0, "nothing learned, no mastery");

		require(near(least(0), 0.09) && near(least(10), 0.54) && near(least(16), 0.81), "where the roll starts");

		require(motion("swingO1", true) == SHOOT && motion("shoot1", true) == SHOOT, "a shot is a shot");
		require(motion("swingO1", false) == SWING && motion("swingTF", false) == SWING && motion("swingPF", false) == SWING, "swings");
		require(motion("stabO1", false) == OTHER && motion("swingD1", false) == OTHER && motion("swingP3", false) == OTHER, "not swings");
		require(motion("proneStab", false) == PRONE && motion("somersault", false) == KICK && motion("fireburner", false) == BURN, "the rest");

		require(forced_swing(1121006, 112, 0, 0.59) && !forced_swing(1121006, 112, 0, 0.6), "Rush three times in five");
		require(forced_swing(1121008, 112, 1, 0.9) && !forced_swing(1121008, 112, 0, 0.0), "Brandish's second hit");
		require(forced_swing(0, 2110, 0, 0.9) && !forced_swing(0, 2000, 0, 0.0), "an Aran's blows, not a Legend's");

		Stats stats{ 100, 50, 30 };
		Formula axe = formula(AXE, 110, 0, SWING, true, stats, 0.5);
		require(near(axe.multiplier, 4.4) && axe.primary == 100 && axe.secondary == 50, "an axe swings for 4.4");
		require(near(formula(AXE, 110, 0, OTHER, false, stats, 0.5).multiplier, 3.2), "and stabs for 3.2");
		require(near(formula(SPEAR, 130, 0, OTHER, false, stats, 0.5).multiplier, 5.0), "a spear stabs for 5.0");
		require(near(formula(POLEARM, 130, 0, SWING, true, stats, 0.5).multiplier, 5.0), "a polearm swings for 5.0");
		require(near(formula(SPEAR, 130, 1311006, OTHER, false, stats, 0.5).multiplier, 4.0), "Dragon Roar is neither");
		require(near(formula(BAREHANDS, 500, 0, OTHER, false, stats, 0.1).multiplier, 3.0) &&
			near(formula(BAREHANDS, 0, 0, OTHER, false, stats, 0.1).multiplier, 4.2), "bare hands");

		Formula dagger = formula(DAGGER, 420, 0, SWING, true, stats, 0.5);
		require(dagger.primary == 30 && near(dagger.multiplier, 3.6) && dagger.secondary == 150, "a thief's dagger");
		require(formula(DAGGER, 110, 0, SWING, true, stats, 0.5).primary == 100, "anyone else's");

		Formula whack = formula(BOW, 300, 0, OTHER, false, stats, 0.5);
		require(whack.primary == 50 && near(whack.multiplier, 3.4) && whack.secondary == 100 && near(whack.divisor, 150.0), "a bow's whack");
		require(near(formula(BOW, 300, 0, SHOOT, false, stats, 0.5).multiplier, 3.4) &&
			near(formula(CROSSBOW, 300, 0, SHOOT, false, stats, 0.5).multiplier, 3.6), "and its shot");
		require(near(formula(CLAW, 400, 0, OTHER, false, stats, 0.5).divisor, 150.0), "a claw's whack");
		require(near(formula(CLAW, 400, 4121008, OTHER, false, stats, 0.5).multiplier, 3.6), "but not Ninja Storm's");
		Formula seven = formula(CLAW, 400, 4001344, SHOOT, false, stats, 0.9);
		require(seven.primary == 30 && near(seven.least, 0.5) && near(seven.multiplier, 5.0) && seven.secondary == 0, "Lucky Seven");
		require(near(formula(GUN, 500, 0, KICK, false, stats, 0.5).multiplier, 1.8), "a kick with a gun");
		require(near(formula(GUN, 500, 0, OTHER, false, stats, 0.5).divisor, 200.0), "a gun's whack");
		require(near(formula(GUN, 500, 0, BURN, false, stats, 0.5).multiplier, 3.6), "Flamethrower fires");
		require(near(formula(GUN, 500, 0, PRONE, false, stats, 0.5).divisor, 200.0) &&
			formula(GUN, 500, 0, PRONE, false, stats, 0.5).primary == 50, "a gun whacks from prone");
		Formula prone = formula(SWORD, 110, 0, PRONE, false, stats, 0.5);
		require(prone.primary == 100 && near(prone.multiplier, 1.0) && prone.secondary == 50 && near(prone.divisor, 200.0), "a prone stab");

		require(near(lowest(axe, 100), (50 * 4.4 + 50) * 100 / 100.0) && near(highest(axe, 100), (100 * 4.4 + 50)), "the range");
		require(near(raw(axe, 100, 0.5), (75 * 4.4 + 50)), "a roll between");

		require(near(capsule(5211004, 2070000), 0.5) && near(capsule(5211004, 2331000), 1.0), "Flamethrower's capsule");
		require(near(capsule(5211005, 2331000), 0.5) && near(capsule(0, 0), 1.0), "Ice Splitter's");
		require(ammo(2060000) && ammo(2070006) && ammo(2330000) && !ammo(2331000), "ammo");
		require(attack(-5) == 0 && attack(2500) == 1999 && attack(120) == 120, "the attack");

		require(near(accuracy(255, 50, 50), 100.0) && near(accuracy(255, 50, 40), 100.0), "no lower against weaker");
		require(near(accuracy(1200, 1, 1), 999 * 100.0 / 255), "at most 999");
		require(near(accuracy(305, 10, 15), 100.0 * 305 / 305), "ten lower a level");
		require(hits(100.0, 70, 0.0) && !hits(100.0, 71, 0.0) && hits(100.0, 130, 1.0) && !hits(100.0, 131, 0.99), "70% to 130%");
		require(hits(5.0, -3, 0.0), "no avoid");
		require(sees(0.2) && !sees(0.21), "Darkness");

		require(near(attribute(100, ms::magic_damage::WEAK, 1.0), 150) && near(attribute(100, ms::magic_damage::STRONG, 1.0), 50), "an element");
		require(near(attribute(100, ms::magic_damage::IMMUNE, 0.5), 50) && near(attribute(100, ms::magic_damage::NORMAL, 3.0), 100), "amplified");
		require(near(attribute(190000, ms::magic_damage::WEAK, 1.0), 199999), "at most 199999 weak");

		require(charge_element(1211004) == 2 && charge_element(1211006) == 1 && charge_element(1211008) == 3, "charges");
		require(charge_element(1221004) == 5 && charge_element(21111005) == 1 && charge_element(1211002) == 0, "and not");
		ms::magic_damage::Attributes attrs = ms::magic_damage::attributes("F3I2");
		require(near(charged(100, { 2, 120, 100 }, attrs), 180) && near(charged(100, { 1, 120, 100 }, attrs), 60), "a charged blow");
		require(near(charged(100, { 0, 120, 100 }, attrs), 100), "no charge");

		require(!defended(1311005) && !defended(4211002) && defended(0), "defense");

		require(critical_skill(BOW, SHOT, 310) == 3000001 && critical_skill(BOW, CLOSE, 310) == 0, "Critical Shot");
		require(critical_skill(CROSSBOW, SHOT, 1310) == 13000000 && critical_skill(CLAW, SHOT, 1410) == 14100001, "Cygnus");
		require(critical_skill(KNUCKLE, CLOSE, 510) == 15110000 && critical_skill(GUN, SHOT, 520) == 0, "knuckles, not guns");

		Critical none;
		Critical sharp = sharpen(none, (15 << 8) | 40);
		require(sharp.rate == 15 && sharp.damage == 140 && sharp.source, "Sharp Eyes alone");
		Critical both = sharpen({ 50, 200, true }, (15 << 8) | 40);
		require(both.rate == 65 && both.damage == 240, "with Critical Throw");
		require(!sharpen(none, 0).source, "no Sharp Eyes");
		Critical stunned = stun(none, 40, 200);
		require(stunned.rate == 40 && stunned.damage == 200, "Stun Mastery");

		require(critical({ 50, 200, true }, 0, 0.49) && !critical({ 50, 200, true }, 0, 0.5), "a critical");
		require(!critical({ 50, 200, false }, 0, 0.0) && !critical({ 50, 200, true }, 4221001, 0.0), "none");

		require(near(barrage(100, 5121007, 4), 200) && near(barrage(100, 15111004, 5), 400) && near(barrage(100, 5121007, 3), 100), "Barrage");

		require(near(scaled(100, 100, 1.0, 0, false, 0), 100), "nothing");
		require(near(scaled(100, 100, 1.5, 120, false, 0), 180), "a skill and combo");
		require(near(scaled(100, 100, 1.5, 0, true, 200), 250), "a critical adds to it");

		std::cout << "physical damage tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";

		return 1;
	}

	return 0;
}
