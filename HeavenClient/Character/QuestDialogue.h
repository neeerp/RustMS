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
		// Numbered pages, in order.
		std::vector<std::string> pages;
		// Shown after the quest was accepted (offer) or completed (completion).
		std::vector<std::string> yes;
		// Shown after the offer was declined.
		std::vector<std::string> no;
		// stop/<page>/answer: the right choice for a question page (choice id).
		std::map<size_t, int32_t> answers;
		// stop/<page>/<choice>: why a wrong choice is wrong.
		std::map<size_t, std::map<int32_t, std::string>> wrong;
		// stop/npc on the completion side: directions while the quest is open.
		std::vector<std::string> reminder;
		// stop/item on the completion side: what to say while the required
		// items are missing (or, for Roger's apples, not yet used up).
		std::vector<std::string> item_reminder;
		// stop/mob on the completion side: what to say while the required
		// mobs have not all been hunted.
		std::vector<std::string> mob_reminder;
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
	// it pages through the conversation and says when to send QUEST_ACTION.
	class QuestDialogue
	{
	public:
		enum class Mode { OFFER, COMPLETION, REMINDER };
		enum class Effect { NONE, START, COMPLETE };

		QuestDialogue(Mode mode, QuestScript script) : mode(mode), script(std::move(script))
		{
			if (mode == Mode::REMINDER)
				enter(Branch::REMINDER);
			else if (pages().empty())
			{
				// Nothing to say: the quest action happens right away.
				initial = mode == Mode::OFFER ? Effect::START : Effect::COMPLETE;
				acted = true;
				enter(Branch::YES);
			}
			else
				enter(Branch::MAIN);
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
			if (branch == Branch::DONE)
				return std::nullopt;

			if (branch == Branch::WRONG)
				return QuestPage { QuestPage::TEXT, wrong_text, false, true };

			const auto& list = current();
			const std::string& text = list[index];

			if (branch == Branch::MAIN)
			{
				if (is_question(index))
					return QuestPage { QuestPage::SELECTION, text, false, false };
				if (mode == Mode::OFFER && index + 1 == list.size())
					return QuestPage { QuestPage::YESNO, text, false, false };
			}

			bool has_next = index + 1 < list.size() ||
				(branch == Branch::MAIN && mode == Mode::COMPLETION && !script.yes.empty());

			// Prev never returns to (or past) an answered question.
			bool has_previous = index > 0 && !(branch == Branch::MAIN && index <= first_after_question());

			return QuestPage { QuestPage::TEXT, text, has_previous, has_next };
		}

		// response: 1 next/yes/select, 0 previous/no, -1 close. `selection` is
		// the chosen #L id on a question page.
		Effect respond(int8_t response, int32_t selection = -1)
		{
			if (branch == Branch::DONE)
				return Effect::NONE;

			if (response < 0)
			{
				branch = Branch::DONE;
				return Effect::NONE;
			}

			if (branch == Branch::WRONG)
			{
				// Back to the same question.
				branch = Branch::MAIN;
				return Effect::NONE;
			}

			if (branch != Branch::MAIN)
			{
				page_through(response);
				return Effect::NONE;
			}

			auto shown = page();

			if (shown->kind == QuestPage::YESNO)
			{
				if (response == 1)
				{
					acted = true;
					enter(Branch::YES);
					return Effect::START;
				}

				enter(Branch::NO);
				return Effect::NONE;
			}

			if (shown->kind == QuestPage::SELECTION)
			{
				if (response != 1)
					return Effect::NONE;

				auto answer = script.answers.find(index);

				if (answer != script.answers.end() && answer->second != selection)
				{
					auto reasons = script.wrong.find(index);

					if (reasons != script.wrong.end() && reasons->second.count(selection))
					{
						wrong_text = reasons->second.at(selection);
						branch = Branch::WRONG;
					}

					return Effect::NONE;
				}

				// Passing the last question completes the quest; what follows
				// is the NPC congratulating the player.
				Effect effect = Effect::NONE;

				if (mode == Mode::COMPLETION && !acted && index == last_question())
				{
					acted = true;
					effect = Effect::COMPLETE;
				}
				else if (mode == Mode::OFFER && !acted && index + 1 == script.pages.size())
				{
					// An offer that ends in a question is accepted by answering it.
					acted = true;
					effect = Effect::START;
				}

				advance();

				return effect;
			}

			if (response == 0)
			{
				if (shown->previous)
					index--;

				return Effect::NONE;
			}

			if (index + 1 < script.pages.size())
			{
				index++;
				return Effect::NONE;
			}

			// Leaving the last completion page.
			Effect effect = Effect::NONE;

			if (mode == Mode::COMPLETION && !acted)
			{
				acted = true;
				effect = Effect::COMPLETE;
			}

			enter(Branch::YES);

			return effect;
		}

		bool finished() const
		{
			return branch == Branch::DONE;
		}

	private:
		enum class Branch { MAIN, YES, NO, WRONG, REMINDER, DONE };

		const std::vector<std::string>& pages() const
		{
			return script.pages;
		}

		const std::vector<std::string>& current() const
		{
			switch (branch)
			{
				case Branch::YES: return script.yes;
				case Branch::NO: return script.no;
				case Branch::REMINDER: return script.reminder;
				default: return script.pages;
			}
		}

		void enter(Branch next)
		{
			branch = next;
			index = 0;

			if (branch != Branch::DONE && current().empty())
				branch = Branch::DONE;
		}

		void advance()
		{
			if (index + 1 < script.pages.size())
				index++;
			else
				enter(Branch::YES);
		}

		void page_through(int8_t response)
		{
			if (response == 0)
			{
				if (index > 0)
					index--;
			}
			else if (index + 1 < current().size())
				index++;
			else
				branch = Branch::DONE;
		}

		bool is_question(size_t page) const
		{
			return script.answers.count(page) ||
				(page < script.pages.size() && script.pages[page].find("#L") != std::string::npos);
		}

		// The first page after the last question, or 0 if there is none.
		size_t first_after_question() const
		{
			for (size_t i = script.pages.size(); i > 0; i--)
				if (is_question(i - 1))
					return i;

			return 0;
		}

		size_t last_question() const
		{
			size_t last = 0;

			for (size_t i = 0; i < script.pages.size(); i++)
				if (is_question(i))
					last = i;

			return last;
		}

		Mode mode;
		QuestScript script;
		Branch branch = Branch::MAIN;
		size_t index = 0;
		bool acted = false;
		Effect initial = Effect::NONE;
		std::string wrong_text;
	};
}
