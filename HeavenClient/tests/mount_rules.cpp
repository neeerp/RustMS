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

		require(z_index("tamingMobMid") < BEHIND && z_index("saddleFront") < BEHIND, "in front");
		require(z_index("saddleRear") == BEHIND && z_index("backTamingMobMid") > BEHIND, "behind");
		require(z_index("characterEnd") == -1 && z_index("") == -1, "neither");

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
