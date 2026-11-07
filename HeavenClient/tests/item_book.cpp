#include "../IO/ItemBook.h"
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
		using namespace ms::item_book;

		require(opens(4161033) && opens(4160000) && opens(4169999), "any 416xxxx opens the book");
		require(!opens(4150000) && !opens(4170000) && !opens(4031739), "other etc items don't");

		// Three pages (4160000): 0|1, then 2 alone.
		require(!has_previous(0) && has_next(0, 2), "the first spread of three has Next");
		require(next(0, 2) == 2 && previous(0) == -1, "Next goes to page 2, Prev nowhere");
		require(has_previous(2) && !has_next(2, 2), "the last spread has Prev only");
		require(next(2, 2) == -1 && previous(2) == 0, "and goes back to 0");

		// One page: no turning.
		require(!has_previous(0) && !has_next(0, 0) && next(0, 0) == -1, "a single page doesn't turn");

		// Two pages: one spread.
		require(!has_next(0, 1) && next(0, 1) == -1, "two pages are one spread");

		// 36 pages (4161033): 0..34 by 2.
		require(next(32, 35) == 34 && has_next(32, 35), "the spread before the last");
		require(next(34, 35) == -1 && !has_next(34, 35), "the last of 36");
		require(previous(35) == 32, "an odd page turns from its spread");

		// Links.
		require(link_target(3, 20) == 2 && link_target(1, 20) == 0 && link_target(7, 20) == 6, "a link goes to its spread");
		require(link_target(21, 20) == -1 && link_target(-1, 20) == -1, "a link past the book goes nowhere");

		require(page_label(0, 35) == "1/36" && page_label(1, 35) == "2/36" && page_label(2, 2) == "3/3", "page numbers");

		require(align(1) == Align::CENTER && align(2) == Align::RIGHT && align(0) == Align::LEFT && align(3) == Align::LEFT,
			"align by number");
		require(align(std::string("1")) == Align::CENTER && align(std::string("1.000000")) == Align::CENTER
			&& align(std::string("2")) == Align::RIGHT && align(std::string("")) == Align::LEFT, "align by string");
	}
	catch (const std::exception& e)
	{
		std::cerr << "item_book: " << e.what() << '\n';
		return 1;
	}

	std::cout << "item_book: ok\n";
	return 0;
}
