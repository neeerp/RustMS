#include "../Gameplay/Combat/HitTiming.h"
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
		using namespace ms;
		using hit_timing::target_delay;

		require(target_delay(Attack::Type::CLOSE, 0, 0) == 0, "the first melee target is hit at once");
		require(target_delay(Attack::Type::CLOSE, 1001005, 1) == 70, "each later melee target 70 ms after the last");
		require(target_delay(Attack::Type::CLOSE, 1001005, 3) == 210, "the fourth melee target 210 ms after the first");
		require(target_delay(Attack::Type::CLOSE, 1001005, 5) == 280, "melee targets wait at most 280 ms");
		require(target_delay(Attack::Type::CLOSE, 4211006, 5) == 0, "Meso Explosion hits every mob at once");
		require(target_delay(Attack::Type::CLOSE, 4211004, 0) == 0, "Band of Thieves's first mob at once");
		require(target_delay(Attack::Type::CLOSE, 4211004, 1) == 280, "and the others 280 ms later");
		require(target_delay(Attack::Type::CLOSE, 4211004, 4) == 280, "all together");
		require(target_delay(Attack::Type::CLOSE, 5101004, 5) == 0, "Corkscrew Blow hits every mob at once");
		require(target_delay(Attack::Type::CLOSE, 15101003, 5) == 0, "the Thunder Breaker's too");
		require(hit_timing::at_once(5101004) && !hit_timing::at_once(5101002), "only Corkscrew Blow lands as it starts");

		using hit_timing::close_hit;
		using hit_timing::CloseAttack;
		using hit_timing::stagger;

		require(stagger(0, 0) == 0 && stagger(0, 2) == 240, "a mob's hits land 120 ms apart");
		require(stagger(4001344, 3) == 360, "as a Lucky Seven's do");
		require(stagger(3111006, 3) == 180 && stagger(13111001, 1) == 60, "Strafe's 60 ms apart");
		require(stagger(5101004, 3) == 0 && stagger(5121007, 3) == 0, "Corkscrew Blow's and Barrage's have their own");

		CloseAttack plain{ 1001005, true, false, 300, 70, 6, 2 };
		require(close_hit(plain, 0) == 370 && close_hit(plain, 1) == 490, "a melee skill's hits follow its first");

		CloseAttack upper{ 5101003, true, false, 300, 70, 6, 2 };
		require(close_hit(upper, 0) == 80 && close_hit(upper, 1) == 470, "Double Uppercut's own times");
		upper.own = false;
		require(close_hit(upper, 1) == 360, "another player's second is sooner");

		CloseAttack barrage{ 5121007, true, false, 300, 70, 6, 6 };
		require(close_hit(barrage, 0) == 450 && close_hit(barrage, 5) == 1980, "Barrage's hits at their table's times");
		barrage.speed = 2;
		require(close_hit(barrage, 1) == 540, "faster with the attack");
		barrage.speed = 6;
		barrage.hits = 12;
		require(close_hit(barrage, 3) == 720, "with a partner's hits, two to a step");

		CloseAttack demolition{ 5121004, true, false, 300, 70, 6, 8 };
		require(close_hit(demolition, 0) == 70 && close_hit(demolition, 2) == 760, "Demolition goes from the start");
		require(close_hit(demolition, 7) == 2080, "its last hit");
		demolition.own = false;
		require(close_hit(demolition, 2) == 610, "another player's are plain");

		CloseAttack shockwave{ 5111006, true, false, 300, 0, 6, 1 };
		require(close_hit(shockwave, 0) == 1380, "Shockwave lands late");

		CloseAttack assassinate{ 4221001, false, false, 300, 0, 6, 3 };
		require(close_hit(assassinate, 0) == 380, "Assassinate's first");
		require(close_hit(assassinate, 1) == 740, "its second 240 ms further on");
		require(close_hit(assassinate, 2) == 940, "its third 80 ms further on");

		CloseAttack brandish{ 1121008, true, false, 300, 0, 6, 2 };
		require(close_hit(brandish, 1) == 660, "a one-handed Brandish");
		brandish.twohanded = true;
		require(close_hit(brandish, 1) == 740, "a two-handed one is slower");
		brandish.own = false;
		require(close_hit(brandish, 1) == 420, "another player's is plain");

		CloseAttack corkscrew{ 5101004, true, false, 0, 0, 6, 3 };
		require(close_hit(corkscrew, 2) == 0, "Corkscrew Blow's all land at once");

		require(target_delay(Attack::Type::MAGIC, 2201005, 0) == 0, "Thunderbolt's first target is hit at once");
		require(target_delay(Attack::Type::MAGIC, 2201005, 5) == 250, "Thunderbolt's targets are 50 ms apart, without a cap");
		require(target_delay(Attack::Type::MAGIC, 2221006, 2) == 200, "Chain Lightning's targets are 100 ms apart");
		require(target_delay(Attack::Type::MAGIC, 2001005, 1) == 0, "other spells hit at once");

		require(target_delay(Attack::Type::RANGED, 0, 2) == 0, "ranged hits are timed by their bullets");

		std::cout << "Hit timing passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
