#include "../Character/Look/CashWeapon.h"
#include <iostream>
#include <string>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::cash_weapon;

		require(type(1302000) == 30, "a one-handed sword is type 30");
		require(type(1332000) == 33, "a dagger 33");
		require(type(1372000) == 37 && type(1382000) == 38, "wands and staves 37 and 38");
		require(type(1472000) == 47 && type(1492000) == 49, "claws 47, guns 49");
		require(type(1342000) == 0 && type(1362000) == 0, "34 to 36 have no type");
		require(type(1702000) == 0, "a cash weapon has none");
		require(type(0) == 0, "nor does no weapon");
		require(type(2302000) == 0, "nor an item outside equips");

		// A cash weapon with art for swords (30) only.
		auto covers = [](int32_t cash, int32_t weapon)
		{
			return cash == 1702000 && type(weapon) == 30;
		};
		auto says = [&](int16_t part, int32_t item, bool cash_item, int32_t weapon, int32_t sticker, int32_t shield,
				const char* start)
		{
			const char* text = refusal(part, item, cash_item, weapon, sticker, shield, covers);
			return start ? text && std::string(text).rfind(start, 0) == 0 : text == nullptr;
		};

		require(says(11, 1702000, true, 0, 0, 0, "You need to equip"), "a cash weapon needs a weapon under it");
		require(says(11, 1702000, true, 1302000, 0, 0, nullptr), "a sword under it is fine");
		require(says(11, 1702000, true, 1472000, 0, 0, "This item is not available"), "a claw isn't");
		require(says(11, 1302001, false, 0, 1702000, 0, nullptr), "a sword goes under a worn cash weapon");
		require(says(11, 1472000, false, 1302000, 1702000, 0, "You may not use the item"), "a claw doesn't");
		require(says(11, 1472000, false, 1302000, 0, 0, nullptr), "with no cash weapon worn, anything goes");
		require(says(10, 1092049, true, 0, 0, 0, "You are able to use this only"), "a cash shield needs a shield");
		require(says(10, 1092049, true, 0, 0, 1092000, nullptr), "and goes on one");
		require(says(10, 1092000, false, 0, 0, 0, nullptr), "a shield needs nothing");
		require(says(5, 1050100, true, 0, 0, 0, nullptr), "other cash equips go on alone");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "cash_weapon tests passed" << std::endl;
	return 0;
}
