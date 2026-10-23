#include "../IO/UiControl.h"
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
		using namespace ms::ui_control;

		constexpr int32_t FIELD = 0, HERO = 112, BEGINNER = 0;

		require(open_window(0, HERO, 120, FIELD) == Window::ITEMS && open_window(1, HERO, 120, FIELD) == Window::EQUIPS &&
			open_window(2, HERO, 120, FIELD) == Window::STATS && open_window(5, HERO, 120, FIELD) == Window::KEYCONFIG,
			"the first windows");
		require(open_window(3, HERO, 120, FIELD) == Window::SKILLS && open_window(0x21, 2200, 20, FIELD) == Window::SKILLS,
			"both skill windows are HC's one");
		require(open_window(6, HERO, 120, FIELD) == Window::QUESTLOG && open_window(9, HERO, 120, FIELD) == Window::MONSTERBOOK &&
			open_window(0xa, HERO, 120, FIELD) == Window::CHARINFO && open_window(0xb, HERO, 120, FIELD) == Window::GUILDBBS,
			"quest log to guild board");
		require(open_window(0x14, HERO, 120, FIELD) == Window::ENERGYBAR && open_window(0x16, HERO, 120, FIELD) == Window::PARTYSEARCH &&
			open_window(0x17, HERO, 120, FIELD) == Window::MAKER && open_window(0x1b, HERO, 120, FIELD) == Window::FAMILY &&
			open_window(0x1c, HERO, 120, FIELD) == Window::FAMILYTREE,
			"energy bar to family tree");
		require(open_window(4, HERO, 120, FIELD) == Window::NONE && open_window(0x12, HERO, 120, FIELD) == Window::NONE &&
			open_window(0x1d, HERO, 120, FIELD) == Window::NONE && open_window(-1, HERO, 120, FIELD) == Window::NONE,
			"windows HC lacks or v83 has none for");

		require(open_window(6, BEGINNER, 1, ARAN_TUTORIAL_FIELD) == Window::QUESTLOG &&
			open_window(0, BEGINNER, 1, ARAN_TUTORIAL_FIELD) == Window::NONE &&
			open_window(3, 2000, 1, ARAN_TUTORIAL_FIELD) == Window::NONE,
			"only the quest log in the Aran tutorial");

		require(open_window(0x1a, BEGINNER, 9, FIELD) == Window::RANKING_NOTICE &&
			open_window(0x1a, 1000, 1, FIELD) == Window::RANKING_NOTICE &&
			open_window(0x1a, 2000, 9, FIELD) == Window::RANKING_NOTICE &&
			open_window(0x1a, 2001, 9, FIELD) == Window::RANKING_NOTICE,
			"the Ranking turns young beginners away");
		require(open_window(0x1a, BEGINNER, 10, FIELD) == Window::NONE && open_window(0x1a, 100, 9, FIELD) == Window::NONE,
			"no Ranking window to open for the rest");

		require(dropped_when_disabled(0xa0) && dropped_when_disabled(0xb9) && dropped_when_disabled(0xcc),
			"spawns and other players' packets go");
		require(!dropped_when_disabled(0xa1) && !dropped_when_disabled(0xb8) && !dropped_when_disabled(0xcd),
			"the rest stays");

		std::cout << "UI control passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
