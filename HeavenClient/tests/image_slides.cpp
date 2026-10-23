#include "../IO/ImageSlides.h"
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
		using namespace ms::image_slides;

		std::vector<std::string> kept = pages({ "", "UI/UIWindow.img/QuestIcon/4/0", "", "Map/Obj/a.img/b/0" });
		require(kept.size() == 2 && kept[0] == "UI/UIWindow.img/QuestIcon/4/0" && kept[1] == "Map/Obj/a.img/b/0",
			"empty paths are not pages");
		require(pages({}).empty() && pages({ "" }).empty(), "no pages");

		Buttons one = opened(1);
		require(!one.prev && !one.next && one.ok(), "a single page has only OK");
		require(!opened(0).next && opened(0).ok(), "no pages still closes");

		Buttons first = opened(3);
		require(!first.prev && first.next && !first.ok(), "the first page has Next");
		Buttons middle = turned(first, 1, 3);
		require(middle.prev && middle.next && !middle.ok(), "a middle page has Prev and Next");
		Buttons last = turned(middle, 2, 3);
		require(last.prev && !last.next && last.ok(), "the last page has Prev and OK");
		Buttons back = turned(last, 1, 3);
		require(back.prev && back.next, "Prev from the last page brings Next back");
		Buttons start = turned(back, 0, 3);
		require(!start.prev && start.next, "back on the first page");

		// Two strings, one empty: v83 opens with Next and keeps it past the one page.
		Buttons padded = opened(2);
		require(padded.next, "Next by the packet's count");
		Buttons beyond = turned(padded, 1, 1);
		require(beyond.prev && beyond.next, "Next stays past the last page");
		require(turned(opened(2), 1, 0).next, "and with no pages at all");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	std::cout << "image slides tests passed\n";
	return 0;
}
