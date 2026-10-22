#include "../Gameplay/MapleMap/WeddingRules.h"
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
		using namespace ms;
		using namespace ms::wedding;

		require(priest_of(CHAPEL) == 9201011, "the Chapel's priest");
		require(priest_of(CATHEDRAL) == 9201002, "the Cathedral's");
		require(priest_of(680000100) == 0, "no priest away from an altar");
		require(bgm_of(CHAPEL) == "BgmGL.img/chapel", "the Chapel's music");
		require(bgm_of(CATHEDRAL) == "BgmGL.img/cathedral", "the Cathedral's");
		require(line_of(1) == "wedding1", "a step's line");

		for (uint8_t step : { 0, 1, 2 })
		{
			require(page_for(CHAPEL, step, true) == Page::SAY, "the Chapel's couple hear each line");
			require(page_for(CHAPEL, step, false) == Page::SAY, "and so do the guests");
		}

		require(page_for(CATHEDRAL, BLESSING, false) == Page::BLESS, "a Cathedral guest may bless");
		require(page_for(CATHEDRAL, BLESSING, true) == Page::NONE, "the couple see nothing then");
		require(page_for(CATHEDRAL, 3, true) == Page::SAY, "then the pronouncement");

		auto [x, y] = hearts_at(100, 200, 140, 210);
		require(x == 120 && y == 185, "the hearts float over the middle");

		Pages pages;
		require(!pages.open(), "nothing open at first");
		require(pages.settle(CHAPEL, 0, true) == 0, "nothing to say then");

		// The welcome answered: one WEDDING_TALK.
		pages.show();
		require(pages.open(), "a page is open");
		require(pages.settle(CHAPEL, 0, true) == 1, "the couple answer it");
		require(!pages.open(), "and it is gone");

		// The welcome still open when the pronouncement comes: both return
		// with the pronouncement.
		pages.show();
		pages.show();
		require(pages.settle(CHAPEL, 1, true) == 2, "each page returns");

		pages.show();
		require(pages.settle(CHAPEL, 1, false) == 0, "a guest says nothing");
		require(!pages.open(), "but the page is gone");

		// The Cathedral's couple stay quiet while the guests bless them.
		pages.show();
		require(pages.settle(CATHEDRAL, BLESSING, true) == 0, "no talk during the blessing");
		pages.show();
		require(pages.settle(CATHEDRAL, 3, true) == 1, "but after it");

		// Another NPC's dialog took the page's place: it never returns.
		pages.show();
		pages.forget();
		require(!pages.open(), "a replaced page is gone");
		require(pages.settle(CHAPEL, 1, true) == 0, "and says nothing");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "wedding rules ok" << std::endl;
	return 0;
}
