#include "../Character/MountRules.h"
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
		using namespace ms::mount;

		require(is_rider_skill(1004) && is_rider_skill(20011004) && !is_rider_skill(1003), "rider skills");
		require(rider_skill(112) == 1004 && rider_skill(1111) == 10001004 && rider_skill(2112) == 20001004,
			"each family its own");
		require(rider_skill(2001) == 20011004 && rider_skill(2218) == 20011004, "Evan's");
		require(ride_effect_skill(1902000, 112) == 1004 && ride_effect_skill(1902005, 1111) == 10001004,
			"a tamed monster shows its rider's Monster Rider");
		require(ride_effect_skill(BATTLESHIP_VEHICLE, 522) == BATTLESHIP, "the ship its own");
		require(ride_effect_skill(1932003, 0) == 0 && ride_effect_skill(0, 112) == 0, "others none");

		require(is_ground_only(5001005) && is_ground_only(10001027) && is_ground_only(20011031),
			"dashes and event mounts keep off ladders");
		require(!is_ground_only(1004) && !is_ground_only(1015) && !is_ground_only(1026) &&
			!is_ground_only(1032) && !is_ground_only(30001027), "Monster Rider doesn't");

		require(!tired(1, 94) && tired(1, 95), "level 1 tires at 95");
		require(!tired(3, 104) && tired(3, 105), "every 3 levels 10 more");

		require(speed(150, 0) == 150 && speed(60, 0) == 80 && speed(180, 20) == 190, "speed");
		require(jump(120, 0) == 120 && jump(70, 5) == 85 && jump(120, 10) == 123, "jump");

		require(action("alert", false, 1902000) == "stand1", "alert stands");
		require(action("swingO1", false, 1902000) == "stand1", "attacks stand");
		require(action("walk2", false, 1902000) == "walk2" && action("prone", true, 1902000) == "prone", "basic");
		require(action("stand2", true, 1902000) == "tired" && action("alert", true, 1902000) == "tired",
			"tired stands tired");
		require(action("rope", false, 1902000) == "rope" && action("ladder", false, 1932003) == "ladder",
			"climbers climb");
		require(action("ladder", false, 1932000) == "stand1", "the Battleship doesn't");
		require(action("cannon", false, 1932000) == "cannon" && action("alert3", true, 1932000) == "alert3",
			"the ship shoots");
		require(action("cannon", false, 1902000) == "stand1", "a tamed monster doesn't");

		require(saddle_fits(1912000, 1902000, { 1902000, 1902001 }), "a saddle fits what it lists");
		require(!saddle_fits(1912000, 1902002, { 1902000, 1902001 }), "and nothing else");
		require(saddle_fits(1912005, 1902105, { 1902005 }), "by the last two digits");
		require(!saddle_fits(1912000, 1902064, { 1902064 }) && saddle_fits(1912000, 1902063, { 1902063 }),
			"of which 64 and up fit nothing");
		require(!saddle_fits(1932000, 1902000, { 1902000 }) && !saddle_fits(1912000, 1932000, { 1932000 }),
			"only saddles and tamed monsters");

		require(is_ship(1932000) && is_ship(1932006) && !is_ship(1932001) && !is_ship(1932011) &&
			!is_ship(1902000), "the ship and not the event vehicles");
		require(ship_allows(5221007) && ship_allows(5201003) && ship_allows(21121008), "guns from the ship");
		require(!ship_allows(BATTLESHIP) && !ship_allows(5001001) && !ship_allows(0), "nothing else");
		require(ship_only(5221008) && !ship_only(5211004), "the Cannon and Torpedo need it");
		require(battleship_max_hp(120, 1) == 400 && battleship_max_hp(200, 30) == 28000, "its HP");

		require(z_index("tamingMobMid") < BEHIND && z_index("saddleFront") < BEHIND, "in front");
		require(z_index("saddleRear") == BEHIND && z_index("backTamingMobMid") > BEHIND, "behind");
		require(z_index("characterEnd") == -1 && z_index("") == -1, "neither");

		auto hog = fixed_info_portrait(1902002);
		require(hog && hog->scale == 70 && hog->x == -25 && hog->y == -25, "the big ones shrink");
		require(fixed_info_portrait(1902017)->y == -15 && !fixed_info_portrait(1902000), "the rest use data");

		require(stays_on(1902040) && stays_on(1902042) && !stays_on(1902039) && !stays_on(1902043), "three stay on");

		Pending pending;
		require(!pending.waiting(0), "nothing asked");
		pending.asked(100);
		pending.asked(200);
		pending.answered();
		require(pending.waiting(7099), "one more due");
		require(!pending.waiting(7100), "given up 7 s after the first");
		pending.answered();
		require(!pending.waiting(7100), "never below none");

		std::cout << "mount rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
