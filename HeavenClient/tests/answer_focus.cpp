#include "../IO/Components/AnswerFocus.h"
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
		using namespace ms::answer_focus;

		require(pressed(Focus::NONE) == Focus::LEFT, "Enter with nothing focused presses YES or ACCEPT");
		require(move(Focus::NONE, false, false) == Focus::NONE, "other keys focus nothing");

		Focus focus = move(Focus::NONE, false, true);
		require(focus == Focus::RIGHT && pressed(focus) == Focus::RIGHT, "Right then Enter presses NO or DECLINE");

		focus = move(focus, true, false);
		require(focus == Focus::LEFT && pressed(focus) == Focus::LEFT, "Left then Enter presses YES or ACCEPT");
		require(move(focus, true, false) == Focus::LEFT, "Left again stays on the left");
		require(move(Focus::RIGHT, false, true) == Focus::RIGHT, "Right again stays on the right");

		std::cout << "Answer focus passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
