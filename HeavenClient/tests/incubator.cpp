#include "../Character/Inventory/Incubator.h"
#include <iostream>
#include <stdexcept>
#include <string>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::incubator;

		require(opens(4170000) && opens(4170009) && opens(4170012), "a double-click on any 417 opens the window");
		require(!opens(4160000) && !opens(4180000) && !opens(INCUBATOR), "nothing else does");
		require(is_egg(4170000) && is_egg(4170009), "the Pigmy Eggs go in");
		require(!is_egg(4170012) && !is_egg(4169999) && !is_egg(4170010), "the DS Egg and the rest don't");
		require(found(2000004, 240)
				== "An item is found inside the Pigmy Egg.#w\r\n\r\n#fUI/UIWindow.img/QuestIcon/4/0#\r\n"
				   "#v2000004:# #t2000004:# 240\r\n\r\n",
				"the popup names the item, its icon and its count");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "incubator tests passed" << std::endl;
	return 0;
}
