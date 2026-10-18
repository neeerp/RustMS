#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// One side of a quest conversation from Quest.nx Say.img/<id>/<0|1>.
	struct QuestScript
	{
		// A question's stop/<page>.
		struct Question
		{
			// stop/<page>/<choice>: why a choice is wrong. A choice without
			// one is right; `answer` means nothing to v83.
			std::map<int32_t, std::string> wrong;
			// stop/<page>/flip: the text before the choices is a page of
			// its own.
			bool flip = false;
		};

		// Numbered pages, in order.
		std::vector<std::string> pages;
		// Shown once the server says the quest was started (offer side) or
		// completed (completion side).
		std::vector<std::string> yes;
		// Shown after the offer was declined.
		std::vector<std::string> no;
		// `ask`: every page but the last is a question.
		bool ask = false;
		std::map<size_t, Question> questions;
		// stop/<reason> (npc, item, quest, mob, default) on the completion
		// side: why the quest can't be completed here, or yet.
		std::map<std::string, std::vector<std::string>> stops;
		// Said instead of stop/npc when a quest item given at the start was
		// lost.
		std::vector<std::string> lost;

		// stop/<reason>, else stop/default, as v83 picks them (FUN_00718b58).
		std::vector<std::string> stop(const std::string& reason) const
		{
			for (const std::string& key : { reason, std::string("default") })
			{
				auto pages = stops.find(key);

				if (pages != stops.end() && !pages->second.empty())
					return pages->second;
			}

			return {};
		}
	};

	struct QuestPage
	{
		enum Kind { TEXT, YESNO, SELECTION };

		Kind kind;
		std::string text;
		bool previous;
		bool next;
	};

	// The client side of the v83 quest flow, independent of UI and assets:
	// it pages through the conversation and says when to send QUEST_ACTION
	// (FUN_00716fe1). Nothing is said after the action; the server's answer
	// brings the yes pages.
	class QuestDialogue
	{
	public:
		enum class Mode { OFFER, COMPLETION, PAGES };
		enum class Effect { NONE, START, COMPLETE };

		// `rewards` goes after the last completion page (FUN_0071808a).
		QuestDialogue(Mode mode, QuestScript script, std::string rewards = {})
			: mode(mode), script(std::move(script)), rewards(std::move(rewards))
		{
			bool silent = mode == Mode::OFFER ? this->script.pages.empty()
				: mode == Mode::COMPLETION && this->script.pages.size() <= 1 && last_page().empty();

			if (silent)
			{
				// Nothing to say: the quest action happens right away.
				initial = mode == Mode::OFFER ? Effect::START : Effect::COMPLETE;
				step = Step::DONE;
			}
			else
			{
				// Rewards with no pages make a last page of their own.
				if (mode == Mode::COMPLETION && this->script.pages.empty())
					this->script.pages.emplace_back();

				enter_page(0);
			}
		}

		// Just pages to read: stop pages, or what the NPC says once a quest
		// is started or completed.
		static QuestDialogue reading(std::vector<std::string> pages)
		{
			QuestScript script;
			script.pages = std::move(pages);

			return QuestDialogue(Mode::PAGES, std::move(script));
		}

		// An action to send as soon as the dialogue opens.
		Effect take_initial_effect()
		{
			Effect effect = initial;
			initial = Effect::NONE;
			return effect;
		}

		// The page to show, or nothing once the dialogue is over.
		std::optional<QuestPage> page() const
		{
			const auto& pages = script.pages;

			switch (step)
			{
				case Step::DONE:
					return std::nullopt;
				case Step::WRONG:
					return QuestPage { QuestPage::TEXT, wrong_text, false, false };
				case Step::DECLINED:
					return QuestPage { QuestPage::TEXT, script.no[index], index > 0, index + 1 < script.no.size() };
				case Step::FLIP:
					return QuestPage { QuestPage::TEXT, pages[index].substr(0, pages[index].find(CHOICES)), index > 0, true };
				case Step::QUESTION:
				{
					std::string text = pages[index];

					if (flipped())
						text = "\r\n" + text.substr(text.find(CHOICES));

					return QuestPage { QuestPage::SELECTION, text, false, false };
				}
				case Step::PAGE:
					break;
			}

			if (mode == Mode::PAGES || index + 1 < pages.size())
				return QuestPage { QuestPage::TEXT, pages[index], index > 0, index + 1 < pages.size() };

			if (mode == Mode::OFFER)
				return QuestPage { QuestPage::YESNO, pages[index], false, false };

			return QuestPage { QuestPage::TEXT, last_page(), false, false };
		}

		// response: 1 next/yes/select, 0 previous/no, -1 close. `selection` is
		// the chosen #L id on a question page.
		Effect respond(int8_t response, int32_t selection = -1)
		{
			auto shown = page();

			if (!shown)
				return Effect::NONE;

			if (response < 0 || step == Step::WRONG)
			{
				step = Step::DONE;
				return Effect::NONE;
			}

			if (step == Step::FLIP)
			{
				// Prev from the flipped page ends the conversation.
				if (response == 1)
					step = Step::QUESTION;
				else
					step = Step::DONE;

				return Effect::NONE;
			}

			if (step == Step::QUESTION)
			{
				auto question = script.questions.find(index);

				if (response != 1 || selection < 0 || question == script.questions.end())
				{
					step = Step::DONE;
					return Effect::NONE;
				}

				auto wrong = question->second.wrong.find(selection);

				if (wrong != question->second.wrong.end() && !wrong->second.empty())
				{
					// The quest is not started or completed; the player can
					// talk to the NPC again.
					wrong_text = wrong->second;
					step = Step::WRONG;
				}
				else
				{
					enter_page(index + 1);
				}

				return Effect::NONE;
			}

			if (step == Step::DECLINED)
			{
				if (response == 0 && shown->previous)
					index--;
				else if (response == 1 && shown->next)
					index++;
				else
					step = Step::DONE;

				return Effect::NONE;
			}

			if (shown->kind == QuestPage::YESNO)
			{
				if (response == 1)
				{
					step = Step::DONE;
					return Effect::START;
				}

				step = Step::DECLINED;
				index = 0;

				if (script.no.empty())
					step = Step::DONE;

				return Effect::NONE;
			}

			if (response == 0)
			{
				if (shown->previous)
					enter_page(index - 1);
				else
					step = Step::DONE;

				return Effect::NONE;
			}

			if (shown->next)
			{
				enter_page(index + 1);
				return Effect::NONE;
			}

			// OK on the last page.
			step = Step::DONE;
			read_to_end = true;

			return mode == Mode::COMPLETION ? Effect::COMPLETE : Effect::NONE;
		}

		bool finished() const
		{
			return step == Step::DONE;
		}

		// Whether the player read every page and left with OK.
		bool read() const
		{
			return read_to_end;
		}

	private:
		enum class Step { PAGE, FLIP, QUESTION, WRONG, DECLINED, DONE };

		static constexpr const char* CHOICES = "#L0#";

		// The last page as shown: on the completion side, with the rewards.
		std::string last_page() const
		{
			std::string text = script.pages.empty() ? std::string() : script.pages.back();

			return mode == Mode::COMPLETION ? text + rewards : text;
		}

		bool question(size_t page) const
		{
			return mode != Mode::PAGES && script.ask && page + 1 < script.pages.size();
		}

		bool flipped() const
		{
			auto question = script.questions.find(index);

			return question != script.questions.end() && question->second.flip &&
				script.pages[index].find(CHOICES) != std::string::npos;
		}

		void enter_page(size_t page)
		{
			index = page;

			if (page >= script.pages.size())
				step = Step::DONE;
			else if (!question(page))
				step = Step::PAGE;
			else
				step = flipped() ? Step::FLIP : Step::QUESTION;
		}

		Mode mode;
		QuestScript script;
		std::string rewards;
		Step step = Step::PAGE;
		size_t index = 0;
		bool read_to_end = false;
		Effect initial = Effect::NONE;
		std::string wrong_text;
	};
}
