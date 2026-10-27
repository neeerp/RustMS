#include "../Character/Inventory/Raise.h"
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
		using namespace ms::raise;

		require(opens(4220046) && opens(4220151) && opens(4220000), "a double-click on any 4220 opens the window");
		require(!opens(4219999) && !opens(4221000) && !opens(4031992), "nothing else does");

		// The Timer's Egg: 1000 a stage, 3 stages.
		Growth timer{1000, 3};
		require(timer.valid() && timer.max() == 3000, "it grows to 3000");
		require(timer.stage(timer.unknown()) == -1, "nothing shows before the record comes");
		require(timer.stage(0) == 0 && timer.stage(999) == 0, "the first stage shows from 0");
		require(timer.stage(1000) == 1 && timer.stage(2999) == 2, "a stage each 1000");
		require(timer.stage(3000) == 3 && timer.stage(5000) == 3, "the last stage is the grade");
		require(timer.feed_message(2900) == 1 && timer.feed_message(3000) == 0, "grown, it takes no more");
		require(!Growth{0, 3}.valid() && !Growth{1000, 0}.valid(), "collection mode and no grade don't open");

		require(ui_path("UI/UIWindow/raise/3") == std::make_pair(std::string("UIWindow.img"), std::string("raise/3")),
				"the art is in the UI file's image");
		require(ui_path("") == std::make_pair(std::string(), std::string()), "no uiData, no art");
		require(ui_path("UI/UIWindow") == std::make_pair(std::string(), std::string()), "nor without a path");
		require(ui_path("Item/raise/3") == std::make_pair(std::string(), std::string()), "nor outside UI");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "raise tests passed" << std::endl;
	return 0;
}
