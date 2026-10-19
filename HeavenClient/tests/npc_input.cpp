#include "../IO/UITypes/NpcInput.h"
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
		using namespace ms::npc_input;

		require(close_answer(0) == -1 && close_answer(1) == -1 && close_answer(12) == -1,
			"say, yes/no and accept/decline close with -1");
		require(close_answer(2) == 0 && close_answer(3) == 0 && close_answer(4) == 0 && close_answer(13) == 0,
			"text, number, menu and box text close with 0");

		require(trim("  Hello there \t\r\n") == "Hello there", "blanks at both ends go");
		require(trim("a  b") == "a  b", "blanks inside stay");
		require(trim(" \t ").empty(), "nothing but blanks is empty");

		require(text_problem("abc", 3).empty(), "the fewest bytes allowed");
		require(text_problem("ab", 3) == "Please enter at least 3 characters.", "too short");
		require(text_problem("", 0).empty(), "an empty answer when none is required");

		std::cout << "NPC input passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
