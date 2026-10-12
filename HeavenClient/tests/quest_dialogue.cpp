#include "../Character/QuestDialogue.h"
#include <iostream>
#include <stdexcept>

using ms::QuestDialogue;
using ms::QuestPage;
using Effect = ms::QuestDialogue::Effect;
using Mode = ms::QuestDialogue::Mode;

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

void offers_page_then_ask_for_acceptance()
{
	ms::QuestScript heena;
	heena.pages = {"hello", "I am Heena", "go see Sera?"};
	heena.yes = {"check the minimap", "Sera is east"};
	heena.no = {"not ready?"};

	QuestDialogue accept(Mode::OFFER, heena);
	require(accept.take_initial_effect() == Effect::NONE, "an offer with pages waits for the player");
	require(accept.page()->kind == QuestPage::TEXT && accept.page()->next, "offer pages have Next");
	require(accept.respond(1) == Effect::NONE, "paging does not start the quest");
	require(accept.respond(0) == Effect::NONE && accept.page()->text == "hello", "Prev goes back");
	accept.respond(1);
	accept.respond(1);
	require(accept.page()->kind == QuestPage::YESNO, "the last offer page asks yes/no");
	require(accept.respond(1) == Effect::START, "Yes starts the quest");
	require(accept.page()->text == "check the minimap", "acceptance pages follow");
	accept.respond(1);
	require(!accept.page()->next, "the last acceptance page has OK");
	require(accept.respond(1) == Effect::NONE && accept.finished(), "OK closes the dialogue");

	QuestDialogue decline(Mode::OFFER, heena);
	decline.respond(1);
	decline.respond(1);
	require(decline.respond(0) == Effect::NONE, "No does not start the quest");
	require(decline.page()->text == "not ready?", "decline pages follow");
	decline.respond(1);
	require(decline.finished(), "decline closes after its pages");

	QuestDialogue closed(Mode::OFFER, heena);
	require(closed.respond(-1) == Effect::NONE && closed.finished(), "closing an offer does nothing");
}

void quiz_answers_complete_once()
{
	ms::QuestScript rain;
	rain.pages = {"What opens the inventory?\\n#L0#I#l#L1#K#l", "That's right!!"};
	rain.answers[0] = 0;
	rain.wrong[0][1] = "K is for skills";

	QuestDialogue quiz(Mode::COMPLETION, rain);
	require(quiz.page()->kind == QuestPage::SELECTION, "the question is a selection");
	require(quiz.respond(1, 1) == Effect::NONE, "a wrong answer pays nothing");
	require(quiz.page()->text == "K is for skills", "a wrong answer explains why");
	quiz.respond(1);
	require(quiz.page()->kind == QuestPage::SELECTION, "the question is asked again");
	require(quiz.respond(1, 7) == Effect::NONE && quiz.page()->kind == QuestPage::SELECTION,
		"an answer without an explanation just asks again");
	require(quiz.respond(1, 0) == Effect::COMPLETE, "the right answer completes the quest");
	require(quiz.page()->text == "That's right!!", "congratulations follow");
	require(!quiz.page()->previous, "no going back to an answered question");
	require(quiz.respond(0) == Effect::NONE && quiz.page()->text == "That's right!!", "Prev is refused");
	require(quiz.respond(1) == Effect::NONE && quiz.finished(), "the quest completes only once");
}

void completions_without_questions_complete_on_the_last_page()
{
	ms::QuestScript sera;
	sera.pages = {"you talked to Heena?"};
	sera.yes = {"Maple Island is small", "use the portal"};
	QuestDialogue visit(Mode::COMPLETION, sera);
	require(visit.page()->next, "more pages follow the greeting");
	require(visit.respond(1) == Effect::COMPLETE, "leaving the last page completes the visit");
	require(visit.page()->text == "Maple Island is small", "completion pages follow");
	require(visit.respond(0) == Effect::NONE && visit.page()->text == "Maple Island is small",
		"Prev stays inside the branch");
	visit.respond(1);
	visit.respond(1);
	require(visit.finished(), "the dialogue ends after the completion pages");

	ms::QuestScript sen;
	QuestDialogue silent(Mode::COMPLETION, sen);
	require(silent.take_initial_effect() == Effect::COMPLETE, "no completion text completes immediately");
	require(silent.finished(), "and shows nothing");

	QuestDialogue silent_offer(Mode::OFFER, ms::QuestScript{});
	require(silent_offer.take_initial_effect() == Effect::START, "no offer text starts immediately");
}

void offers_ending_in_a_question_start_when_answered()
{
	ms::QuestScript test;
	test.pages = {"Are you brave?\\n#L0#Yes#l#L1#No#l"};
	test.answers[0] = 0;
	QuestDialogue offer(Mode::OFFER, test);
	require(offer.respond(1, 1) == Effect::NONE, "the wrong answer does not start it");
	require(offer.respond(1, 0) == Effect::START, "the right answer starts it");
}

void reminders_only_page()
{
	ms::QuestScript heena;
	heena.reminder = {"Sera is east"};
	QuestDialogue reminder(Mode::REMINDER, heena);
	require(reminder.page()->text == "Sera is east", "reminders show stop/npc");
	require(reminder.respond(1) == Effect::NONE && reminder.finished(), "reminders have no effect");
	QuestDialogue empty(Mode::REMINDER, ms::QuestScript{});
	require(empty.finished(), "no reminder text shows nothing");
}

int main()
{
	try
	{
		offers_page_then_ask_for_acceptance();
		quiz_answers_complete_once();
		completions_without_questions_complete_on_the_last_page();
		offers_ending_in_a_question_start_when_answered();
		reminders_only_page();
		std::cout << "Quest dialogue passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
