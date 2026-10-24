#include "../IO/UITypes/ChatLogLayout.h"
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
		using namespace ms::chat_log;

		require(top(Mode::EXPANDED, 72) == -70 && top(Mode::EXPANDED, 72) + 72 == 2,
				"the expanded log ends 2 pixels into the status bar");
		require(top(Mode::TYPING, 24) == -21 && top(Mode::LINE, 24) == 6, "the one-line modes' layers");

		require(lift(Mode::EXPANDED) == 0 && lift(Mode::TYPING) == 5 && lift(Mode::LINE) == 5,
				"only the one-line modes lift their text");

		Strip bottom = strip(Mode::EXPANDED, 72, 0);
		Strip second = strip(Mode::EXPANDED, 72, 1);
		require(bottom.top == 58 && bottom.height == 13 && second.top == 45, "an expanded row's strip starts a pixel up");

		Strip line = strip(Mode::LINE, 24, 0);
		require(line.top == 1 && line.top + line.height == 24, "a one-line strip runs to the layer's bottom");

		require(strip_width(11) == 555, "strips stop 14 pixels short of the scroll bar");

		std::cout << "Chat log layout passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
