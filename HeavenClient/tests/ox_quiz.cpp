#include "../Gameplay/OxQuizRules.h"

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
		using namespace ms::ox_quiz;

		require(show(1, 3, true, true, true) == Show::QUESTION, "a flag shows the question");
		require(show(0, 3, true, true, true) == Show::ANSWER, "no flag shows the answer");
		require(show(1, 0, true, true, true) == Show::CLEAR, "question 0 takes it down");
		require(show(0, 0, false, false, false) == Show::CLEAR, "before anything is looked up");
		require(show(1, 3, false, false, false) == Show::CLEAR, "no OXQuiz.img takes it down");
		require(show(1, 3, true, false, false) == Show::KEEP, "a missing set leaves it");
		require(show(1, 200, true, true, false) == Show::CLEAR, "a missing question takes it down");

		// Set 1's first question (Etc.nx OXQuiz.img/1/1).
		require(answer_line("The weakest monster is the GREEN SNAIL. (LV. 1)", 0) ==
					"The weakest monster is the GREEN SNAIL. (LV. 1) [Answer:X]",
				"an explanation and X");
		require(answer_line("", 1) == "[Answer:O]", "no explanation, no space");
		require(answer_line("d", 7) == "d [Answer:O]", "any answer but 0 is O");

		// The help lines: Map.img's help0 and help1 of an event map.
		auto helps = [](uint8_t i) { return i < 2 ? "help" + std::to_string(i) : std::string(); };

		require(ms::gm_event::help_line(0, helps) == "help0", "the first line");
		require(ms::gm_event::help_line(1, helps) == "help1", "the second line");
		require(ms::gm_event::help_line(2, helps).empty(), "past the last line");

		auto gap = [](uint8_t i) { return i == 1 ? std::string() : "help" + std::to_string(i); };

		require(ms::gm_event::help_line(2, gap).empty(), "a line after a missing one");
		require(ms::gm_event::help_line(0, gap) == "help0", "a line before the gap");

		std::cout << "ox quiz tests passed\n";
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
