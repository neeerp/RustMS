#include "../Gameplay/MapleMap/GuideRules.h"
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
		using namespace ms::guide_rules;

		require(skill_for(1000) == CYGNUS && skill_for(1100) == CYGNUS && skill_for(1511) == CYGNUS,
			"Cygnus Knights get the fairy");
		require(skill_for(0) == OTHERS && skill_for(2000) == OTHERS && skill_for(2001) == OTHERS
			&& skill_for(112) == OTHERS, "everyone else gets the other guide");

		require(says_after_hint(1000) && says_after_hint(0), "a hint ends with say");
		require(!says_after_hint(2000) && !says_after_hint(2001) && !says_after_hint(2100),
			"but not for Aran or Evan");

		require(leaves(10, 11), "level 11 sends it away");
		require(!leaves(11, 11) && !leaves(11, 12) && !leaves(9, 10), "only reaching 11 does");
		require(!leaves(12, 11), "losing levels does not");

		require(!expired(0, 0) && expired(30, 0), "a zero duration lasts one update");
		require(!expired(3000, 3000) && expired(3030, 3000), "it stays its whole duration");

		ms::Point<int16_t> corner = balloon_corner(200, 28, 80);
		require(corner.x() == -100 && corner.y() == -198, "the balloon's top left");

		ms::Point<int16_t> arrow = balloon_arrow(200, 28);
		require(arrow.x() == 110 && arrow.y() == 63, "the arrow under the text");

		ms::Point<int16_t> hint = hint_corner(ms::Point<int16_t>(215, 283), 70);
		require(hint.x() == -107 && hint.y() == -353, "a hint's top left");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "guide rules tests passed\n";
	return 0;
}
