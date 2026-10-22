#include "../Character/FairyBlessing.h"
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
		using namespace ms::fairy_blessing;

		require(is_blessing(12) && is_blessing(10000012) && is_blessing(20000012) && is_blessing(20010012),
				"the four blessings");
		require(!is_blessing(1000) && !is_blessing(10000013) && !is_blessing(0), "nothing else");
		require(description("Sib") ==
					"Spirits who receive #cSib#'s energy will give your character a boost. 1 Skill Point will be "
					"granted each time #cSib#'s level increases by 10.",
				"0x1449 with the name twice");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	return 0;
}
