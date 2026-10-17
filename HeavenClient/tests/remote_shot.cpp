#include "../Gameplay/Combat/RemoteShot.h"

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
		using namespace ms::remote_shot;

		require(attack_delay(0, 400) == 400 && attack_delay(3101005, 400) == 400, "most shots hit at the action's delay");
		require(attack_delay(5221008, 400) == 600 && attack_delay(5210000, 400) == 90
				&& attack_delay(13111006, 400) == 618 && attack_delay(15111007, 400) == 449,
				"a few have their own");

		require(start(0, { 100, 200 }, true, false) == Point<int16_t>(50, 172), "a shot to the left starts 50 px left");
		require(start(0, { 100, 200 }, false, true) == Point<int16_t>(150, 172), "to the right, 50 px right");
		require(start(15111007, { 100, 200 }, false, false).x() == 250, "Shark Wave starts 150 px ahead");
		require(start(11101004, { 100, 200 }, false, true).x() == 30, "Soul Blade 70 px the way the user faces");

		require(end({ 0, 0 }, { 300, 0 }) == Point<int16_t>(300, 0), "a bullet flies to where it aims");
		require(end({ 0, 0 }, { 800, 0 }) == Point<int16_t>(400, 0), "but no further than 400 px");
		require(end({ 0, 0 }, { -600, 800 }) == Point<int16_t>(-240, 320), "along the way");

		require(hit_delay(0, 0, 0, 400, { 0, 0 }, { 300, 400 }) == 750, "hits land 1.5 ms per pixel to the aim");
		require(hit_delay(0, 0, 0, 400, { 0, 0 }, { 1000, 0 }) == 1500, "all the way, past the bullet's end");
		require(hit_delay(3101005, 2, 300, 400, { 0, 0 }, { 10, 0 }) == 500, "Arrow Bomb's splash 200 ms on");
		require(hit_delay(3111003, 2, 300, 400, { 0, 0 }, { 10, 0 }) == 480, "Inferno's 30 ms apart from 120 on");
		require(hit_delay(3111004, 0, 0, 400, { 0, 0 }, { 100, 0 }) == 150, "Arrow Rain's first mob as any shot");
		require(hit_delay(3111004, 1, 150, 400, { 0, 0 }, { 100, 0 }) == 800, "its others a delay and 400 ms on");

		require(shown_hits(3, 1, 1) == 1, "a single bullet carries one hit");
		require(shown_hits(4, 4, 1) == 4 && shown_hits(2, 3, 1) == 2, "Strafe's four bullets four");
		require(shown_hits(5, 0, 0) == 1, "counts are at least 1");

		Rectangle<int16_t> none;
		Rectangle<int16_t> area{ -200, 200, -100, 10 };
		Rectangle<int16_t> mob{ 160, 220, 100, 200 };

		require(shoots(0, 0, none, 150, mob, false), "a shot flies a bullet to its first mob");
		require(!shoots(0, 1, none, 150, { 300, 360, 100, 200 }, false), "but not to the later ones");
		require(shoots(3121004, 1, none, 150, { 300, 360, 100, 200 }, false), "Hurricane to every one");
		require(shoots(5221004, 2, none, 150, { 300, 360, 100, 200 }, false), "and Rapid Fire");
		require(!shoots(3001004, 0, none, 150, { 140, 200, 100, 200 }, false),
				"a skill with no area not to a mob reaching back past the start");
		require(shoots(0, 0, none, 150, { 140, 200, 100, 200 }, false), "a plain shot still does");
		require(shoots(3101005, 0, area, 150, { 140, 200, 100, 200 }, false), "and so does a skill with an area");
		require(!shoots(3111004, 0, area, 150, mob, false), "Arrow Rain draws no bullet");
		require(!shoots(15111007, 0, none, 150, mob, false) && !shoots(5121002, 0, none, 150, mob, false),
				"nor Shark Wave, nor Energy Orb here");

		require(!hides_miss(15111007) && hides_miss(3111004) && hides_miss(15111006),
				"a miss hides fewer bullets");
		require(miss_range(false) == 200 && miss_range(true) == 300, "a miss flies 200 px, 300 from a bow");
		require(miss_start(0, 100, false) == 150 && miss_start(15111007, 100, true) == -50, "from 50 px ahead");
		require(miss_flight(0, 150, 300) == 225 && miss_flight(21100004, 150, 300) == 150,
				"1.5 ms per pixel, 1 for Combo Smash");
		require(miss_spread(0, 1) == 0 && miss_spread(0, 2) == -4 && miss_spread(1, 2) == 4,
				"its bullets 8 px apart around the row");

		std::cout << "Remote shot passed\n";
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	return 0;
}
