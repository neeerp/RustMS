#include "../Gameplay/MapleMap/FieldLimit.h"
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
		using namespace ms::field_limit;

		// The eleven maps that list items all have fieldLimit 0x526fc.
		const std::vector<int32_t> allowed = { 2022539, 2109004 };

		require(may_use(0, {}, 2000000), "a Red Potion works on an open map");
		require(!may_use(0x400, {}, 2000000), "a Red Potion is refused where consumables are limited");
		require(!may_use(0x526fc, allowed, 2000000), "an unlisted potion is refused");
		require(may_use(0x526fc, allowed, 2022539), "a listed item still works");
		require(may_use(0x400, {}, 2030000), "a Return Scroll doesn't go the potion way");
		require(may_use(0x400, {}, 2040000), "a scroll doesn't go the potion way");
		require(!may_use(0x400, {}, 2210000), "a transformation potion is refused");

		require(may_drop(0x400), "consumable limits leave drops alone");
		require(!may_drop(0x400000), "bit 0x400000 keeps items on the player");

		std::cout << "Field limit passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
