#include "../Character/MonsterBook.h"
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

		MonsterBook book;

		require(book.get_book_level() == 1, "an empty book is level 1");

		book.add_card(0, 5);
		book.add_card(8000, 1);
		book.set_card(2380001, 2);

		require(book.get_level(2380000) == 5, "set-field cards go by their offset from the Snail card");
		require(book.get_level(2380002) == 0, "a missing card has no copies");
		require(book.count_cards(false) == 2, "two basic cards");
		require(book.count_cards(true) == 1, "category 8 holds the special cards");

		for (int32_t card = 2381000; book.count_cards(false) + book.count_cards(true) < 10; card++)
			book.set_card(card, 1);

		require(book.get_book_level() == 1, "ten different cards are still level 1");

		book.set_card(2382000, 1);

		require(book.get_book_level() == 2, "the eleventh card makes level 2");

		for (int32_t card = 2383000; book.count_cards(false) + book.count_cards(true) < 31; card++)
			book.set_card(card, 1);

		require(book.get_book_level() == 3, "31 cards make level 3");

		std::cout << "Monster Book passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
