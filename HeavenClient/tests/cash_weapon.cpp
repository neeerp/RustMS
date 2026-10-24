#include "../Character/Look/CashWeapon.h"
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
		using namespace ms::cash_weapon;

		require(type(1302000) == 30, "a one-handed sword is type 30");
		require(type(1332000) == 33, "a dagger 33");
		require(type(1372000) == 37 && type(1382000) == 38, "wands and staves 37 and 38");
		require(type(1472000) == 47 && type(1492000) == 49, "claws 47, guns 49");
		require(type(1342000) == 0 && type(1362000) == 0, "34 to 36 have no type");
		require(type(1702000) == 0, "a cash weapon has none");
		require(type(0) == 0, "nor does no weapon");
		require(type(2302000) == 0, "nor an item outside equips");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "cash_weapon tests passed" << std::endl;
	return 0;
}
