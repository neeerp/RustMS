#include "../Character/QuestDialogue.h"
#include "../Character/QuestRewards.h"
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
	heena.no = {"not ready?", "come back later"};

	QuestDialogue accept(Mode::OFFER, heena);
	require(accept.take_initial_effect() == Effect::NONE, "an offer with pages waits for the player");
	require(accept.page()->kind == QuestPage::TEXT && accept.page()->next && !accept.page()->previous,
		"the first offer page has only Next");
	require(accept.respond(1) == Effect::NONE && accept.page()->previous, "paging does not start the quest");
	require(accept.respond(0) == Effect::NONE && accept.page()->text == "hello", "Prev goes back");
	accept.respond(1);
	accept.respond(1);
	require(accept.page()->kind == QuestPage::YESNO, "the last offer page asks yes/no");
	require(accept.respond(1) == Effect::START, "Yes starts the quest");
	require(accept.finished(), "the yes pages wait for the server");

	QuestDialogue decline(Mode::OFFER, heena);
	decline.respond(1);
	decline.respond(1);
	require(decline.respond(0) == Effect::NONE, "No does not start the quest");
	require(decline.page()->text == "not ready?" && decline.page()->next, "decline pages follow");
	decline.respond(1);
	require(decline.page()->previous && !decline.page()->next, "the last decline page has Prev and OK");
	decline.respond(1);
	require(decline.finished(), "decline closes after its pages");

	QuestDialogue closed(Mode::OFFER, heena);
	require(closed.respond(-1) == Effect::NONE && closed.finished(), "closing an offer does nothing");

	QuestDialogue silent_offer(Mode::OFFER, ms::QuestScript{});
	require(silent_offer.take_initial_effect() == Effect::START && silent_offer.finished(),
		"no offer text starts immediately");
}

void wrong_answers_end_the_conversation()
{
	// Say.img/1009/1: Rain's quiz.
	ms::QuestScript rain;
	rain.ask = true;
	rain.pages = {"What opens the inventory?\\n#L0#I#l#L1#K#l#L2#S#l", "That's right!!"};
	rain.questions[0].wrong[1] = "K is for skills";

	QuestDialogue wrong(Mode::COMPLETION, rain, "#wrewards#w");
	require(wrong.page()->kind == QuestPage::SELECTION && !wrong.page()->previous, "a question has no Prev");
	require(wrong.respond(1, 1) == Effect::NONE, "a wrong answer pays nothing");
	require(wrong.page()->text == "K is for skills" && !wrong.page()->next && !wrong.page()->previous,
		"a wrong answer explains why, with OK");
	require(wrong.respond(1) == Effect::NONE && wrong.finished(), "and ends the conversation");

	QuestDialogue right(Mode::COMPLETION, rain, "#wrewards#w");
	require(right.respond(1, 2) == Effect::NONE, "an answer without an explanation is right");
	require(right.page()->text == "That's right!!#wrewards#w", "the last page carries the rewards");
	require(!right.page()->previous && !right.page()->next, "the last completion page has only OK");
	require(right.respond(1) == Effect::COMPLETE && right.finished() && right.read(), "OK completes the quest");

	QuestDialogue closed(Mode::COMPLETION, rain);
	closed.respond(1, 0);
	require(closed.respond(-1) == Effect::NONE && closed.finished(), "closing the last page completes nothing");

	ms::QuestScript unknown = rain;
	unknown.questions.clear();
	QuestDialogue lost(Mode::COMPLETION, unknown);
	require(lost.respond(1, 0) == Effect::NONE && lost.finished(), "a question without stop/<page> ends the conversation");
}

void flipped_questions_say_their_lead_first()
{
	// Say.img/2270/1: Lana's questions.
	ms::QuestScript lana;
	lana.ask = true;
	lana.pages = {"Are you here to be a star?\r\n#L0#No, for Blake.#l", "Blake?\r\n#L0#Give him a chance.#l", "I'll think."};
	lana.questions[0].flip = true;
	lana.questions[1].flip = true;

	QuestDialogue offer(Mode::OFFER, lana);
	QuestPage lead = *offer.page();
	require(lead.kind == QuestPage::TEXT && lead.text == "Are you here to be a star?\r\n", "the lead comes first");
	require(lead.next && !lead.previous, "the first lead has only Next");
	require(lead.speaker == 0, "Lana says the lead");
	offer.respond(1);
	require(offer.page()->kind == QuestPage::SELECTION && offer.page()->text == "\r\n#L0#No, for Blake.#l",
		"then the choices on their own");
	require(offer.page()->speaker == 2, "which the player says");
	offer.respond(1, 0);
	require(offer.page()->text == "Blake?\r\n" && offer.page()->previous, "the next lead has Prev");
	require(offer.respond(0) == Effect::NONE && offer.finished(), "Prev from a lead ends the conversation");

	QuestDialogue accept(Mode::OFFER, lana);
	accept.respond(1);
	accept.respond(1, 0);
	accept.respond(1);
	accept.respond(1, 0);
	require(accept.page()->kind == QuestPage::YESNO, "an offer's last question page asks yes/no");
	require(accept.respond(1) == Effect::START, "Yes starts it");
}

void completions_page_to_the_rewards()
{
	ms::QuestScript sera;
	sera.pages = {"you talked to Heena?", "here you go"};
	sera.yes = {"Maple Island is small"};
	QuestDialogue visit(Mode::COMPLETION, sera, "#w+exp#w");
	require(visit.page()->next && visit.page()->text == "you talked to Heena?", "pages before the last have Next");
	require(visit.respond(1) == Effect::NONE, "paging completes nothing");
	require(visit.page()->text == "here you go#w+exp#w" && !visit.page()->previous, "the last page has no Prev");
	require(visit.respond(1) == Effect::COMPLETE && visit.finished(), "OK completes the quest");

	QuestDialogue silent(Mode::COMPLETION, ms::QuestScript{});
	require(silent.take_initial_effect() == Effect::COMPLETE && silent.finished(), "nothing to say completes at once");

	QuestDialogue rewards_only(Mode::COMPLETION, ms::QuestScript{}, "#w+exp#w");
	require(rewards_only.take_initial_effect() == Effect::NONE, "rewards alone are still shown");
	require(rewards_only.page()->text == "#w+exp#w", "on a page of their own");
}

void rewards_are_listed_under_the_last_page()
{
	ms::QuestRewards rewards;
	rewards.exp = 30;
	rewards.items = {
		{ 2000000, 5 },
		{ 4031802, -1 },
		{ 1002053, 1, 1 << 1 },
		{ 1041002, 1, -1, 1 },
		{ 1002008, 1, -1, 2, 1 }
	};

	const std::string HEADER = "#w\r\n\r\n#fUI/UIWindow.img/QuestIcon/4/0#\r\n";
	const std::string EXP = "#fUI/UIWindow.img/QuestIcon/8/0# 30 exp\r\n\r\n";
	const std::string RANDOM = "#FUI/UIWindow.img/QuestIcon/5/0#\r\n";

	require(rewards.summary(0, false, false) == HEADER + "#v2000000:# #t2000000:# 5\r\n\r\n" + EXP + RANDOM + "#w",
		"items taken, or for other jobs or genders, are left out; random ones are not named");
	require(rewards.summary(100, true, false) ==
			HEADER + "#v2000000:# #t2000000:# 5\r\n#v1002053:# #t1002053:# 1\r\n#v1041002:# #t1041002:# 1\r\n\r\n" + EXP +
				RANDOM + "#w",
		"a warrior's hat and a woman's shirt");
	require(rewards.summary(900, false, false).find("1002053") != std::string::npos, "GMs pass every job filter");
	require(rewards.summary(0, false, true).empty(), "nothing is listed when another quest follows");
	require(ms::QuestRewards{}.summary(0, false, false).empty(), "or when there is nothing to list");

	ms::QuestRewards fame;
	fame.fame = 2;
	require(fame.summary(0, false, false) == HEADER + "\r\n#fUI/UIWindow.img/QuestIcon/6/0# 2\r\n\r\n#w",
		"fame is listed after EXP");
	fame.fame = -2;
	require(fame.summary(0, false, false).empty(), "a loss of fame is not");
}

void pages_are_read_to_the_end()
{
	QuestDialogue yes = QuestDialogue::reading({"well done", "see Sera"});
	require(yes.page()->next && !yes.page()->previous, "the first page has Next");
	yes.respond(1);
	require(yes.page()->previous && !yes.page()->next, "the last page has Prev and OK");
	require(yes.respond(1) == Effect::NONE && yes.finished() && yes.read(), "OK on the last page reads them all");

	QuestDialogue closed = QuestDialogue::reading({"well done", "see Sera"});
	closed.respond(-1);
	require(closed.finished() && !closed.read(), "closing early does not");

	require(QuestDialogue::reading({}).finished(), "no pages show nothing");

	ms::QuestScript script;
	script.stops["default"] = {"not yet"};
	script.stops["item"] = {"bring the apples"};
	script.stops["mob"] = {};
	require(script.stop("item") == std::vector<std::string>{"bring the apples"}, "stop/<reason> first");
	require(script.stop("mob") == std::vector<std::string>{"not yet"}, "then stop/default");
	require(ms::QuestScript{}.stop("npc").empty(), "or nothing");
}

int main()
{
	try
	{
		offers_page_then_ask_for_acceptance();
		wrong_answers_end_the_conversation();
		flipped_questions_say_their_lead_first();
		completions_page_to_the_rewards();
		rewards_are_listed_under_the_last_page();
		pages_are_read_to_the_end();
		std::cout << "Quest dialogue passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
