#include "../IO/WindowLayer.h"
#include <iostream>
#include <list>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

enum Type { STATUSBAR, CHATBAR, ITEMS, CARD, NOTICE };

bool is_top(Type type)
{
	return type == CARD || type == NOTICE;
}

int main()
{
	try
	{
		using ms::window_layer::raise;

		std::list<Type> order;
		raise(order, STATUSBAR, is_top);
		raise(order, CARD, is_top);
		raise(order, CHATBAR, is_top);
		require(order == std::list<Type>{ STATUSBAR, CHATBAR, CARD }, "a window added after a card goes under it");

		raise(order, STATUSBAR, is_top);
		require(order == std::list<Type>{ CHATBAR, STATUSBAR, CARD }, "clicking the status bar keeps the card above it (#608)");

		raise(order, NOTICE, is_top);
		require(order == std::list<Type>{ CHATBAR, STATUSBAR, CARD, NOTICE }, "a dialog opened after a card covers it");

		raise(order, CARD, is_top);
		require(order == std::list<Type>{ CHATBAR, STATUSBAR, NOTICE, CARD }, "clicking a card raises it among its own layer");

		raise(order, ITEMS, is_top);
		require(order == std::list<Type>{ CHATBAR, STATUSBAR, ITEMS, NOTICE, CARD }, "a new window stays under the top layer");

		std::cout << "Window layer passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
