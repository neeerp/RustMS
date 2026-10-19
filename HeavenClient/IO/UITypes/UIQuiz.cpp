#include "UIQuiz.h"

#include "NpcInput.h"
#include "Quiz.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cstdio>

namespace ms
{
	namespace
	{
		// Everything is from the window's top left. The countdown's digits and comma (7e9dc6 to 7ea62d,
		// 7eb847 to 7ec0ae).
		constexpr int16_t DIGITS_Y = 33;
		constexpr int16_t DIGIT_X[4] = { 111, 132, 168, 189 };
		constexpr int16_t COMMA_X = 155;
		// The box takes at most 50 bytes (7e9c3d, 7eb57c).
		constexpr size_t BOX_LIMIT = 50;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9 for 12 px text and 10 for 15 px
		// bold, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr int16_t TITLE_RAISE = 7;

		// The quiz (7e9bc4, 7e9d54).
		constexpr int16_t QUIZ_WIDTH = 288, QUIZ_HEIGHT = 229;
		constexpr int16_t TITLE_Y = 80;
		const Point<int16_t> QUESTION_LABEL_AT(52, 110);
		const Point<int16_t> QUESTION_AT(92, 110);
		const Point<int16_t> CLUE_LABEL_AT(52, 130);
		const Point<int16_t> CLUE_AT(92, 130);
		const Point<int16_t> ENTER_ANSWER_AT(45, 157);
		const Point<int16_t> HINT_AT(38, 202);
		const Point<int16_t> QUIZ_BOX_AT(109, 157);
		constexpr int16_t QUIZ_BOX_WIDTH = 150;
		const Point<int16_t> QUIZ_OK_AT(241, 199);

		// The speed quiz (7eb516, 7eb7d5).
		constexpr int16_t SPEED_WIDTH = 285, SPEED_HEIGHT = 409;
		const Point<int16_t> POINTS_AT(237, 45);
		const Point<int16_t> NUMBER_AT(99, 381);
		// Where the picture's origin goes, 16 to the left for an item (7ec34d).
		const Point<int16_t> PICTURE_AT(156, 287);
		constexpr int16_t ITEM_SHIFT = 16;
		const Point<int16_t> SPEED_BOX_AT(100, 339);
		constexpr int16_t SPEED_BOX_WIDTH = 165;
		const Point<int16_t> SPEED_OK_AT(233, 380);
		const Point<int16_t> NEXT_AT(190, 380);
		const Point<int16_t> GIVE_UP_AT(126, 380);

		// The box is 13 high, its text raised like the rest.
		Rectangle<int16_t> box_at(Point<int16_t> at, int16_t width)
		{
			return Rectangle<int16_t>(at.x(), at.x() + width, at.y() - TEXT_RAISE, at.y() + 13);
		}

		std::string format(const char *pattern, int32_t value)
		{
			char text[32];
			std::snprintf(text, sizeof(text), pattern, value);

			return text;
		}

		enum Buttons : uint16_t
		{
			OK,
			NEXT,
			GIVE_UP
		};
	}

	UIQuizWindow::UIQuizWindow(nl::node src, int8_t type, int32_t seconds, Rectangle<int16_t> box)
			: UIElement(Point<int16_t>(), Texture(src["backgrnd"]).get_dimensions()), msgtype(type),
			  opened(ContinuousTimer::get().start()), total(seconds), shown(-1), answered(false)
	{
		sprites.emplace_back(src["backgrnd"]);

		for (int32_t i = 0; i < 10; i++)
			digits[i] = src["num1"][std::to_string(i)];

		comma = src["num1"]["comma"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"]);

		// The box has the keyboard from the start (7e9c9a, 7eb5dc).
		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, box, BOX_LIMIT);
		field.set_enter_callback([this](std::string) { submit(); });
		field.set_state(Textfield::State::FOCUSED);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	UIQuizWindow::~UIQuizWindow()
	{
		if (field.get_state() == Textfield::State::FOCUSED)
			UI::get().remove_textfield();
	}

	void UIQuizWindow::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		std::array<int32_t, 4> countdown = quiz::countdown(std::max(shown, 0));

		for (size_t i = 0; i < countdown.size(); i++)
			if (countdown[i] < 10)
				digits[countdown[i]].draw(position + Point<int16_t>(DIGIT_X[i], DIGITS_Y));

		comma.draw(position + Point<int16_t>(COMMA_X, DIGITS_Y));
	}

	void UIQuizWindow::update()
	{
		UIElement::update();
		field.update(position);

		int64_t left_ms = total * 1000LL - ContinuousTimer::get().stop(opened) / 1000;
		int32_t seconds = quiz::seconds_left(left_ms);

		if (seconds == shown)
			return;

		shown = seconds;
		tick(seconds);

		// The time is up: what was typed is lost, and an empty answer goes (7eb1c9, 7ec885).
		if (seconds <= 0)
			answer("");
	}

	// Centred on the screen, which for v83 is always 800 by 600 (7e9ac1, 7eb409).
	void UIQuizWindow::update_screen(int16_t new_width, int16_t new_height)
	{
		position = Point<int16_t>((new_width - dimension.x()) / 2, (new_height - dimension.y()) / 2);
	}

	Cursor::State UIQuizWindow::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// Clicking the box gives it the keyboard back.
		if (field.get_state() == Textfield::State::NORMAL)
			if (Cursor::State fstate = field.send_cursor(cursorpos, clicked); fstate != Cursor::State::IDLE)
				return fstate;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIQuizWindow::hide()
	{
		answered = true;
		deactivate();

		if (field.get_state() == Textfield::State::FOCUSED)
			UI::get().remove_textfield();
	}

	void UIQuizWindow::focus()
	{
		if (is_active())
			field.set_state(Textfield::State::FOCUSED);
	}

	void UIQuizWindow::answer(const std::string &text)
	{
		if (answered)
			return;

		hide();
		NpcQuizAnswerPacket(msgtype, text).dispatch();
	}

	int32_t UIQuizWindow::get_total() const
	{
		return total;
	}

	UIQuiz::UIQuiz(const std::string &t, const std::string &q, const std::string &c, int32_t min, int32_t max,
				   int32_t seconds)
			: UIQuizWindow(NxFiles::ui_classic["UIWindow.img"]["InitialQuiz"], 5, seconds,
						   box_at(QUIZ_BOX_AT, QUIZ_BOX_WIDTH)), min_letters(min), max_letters(max)
	{
		buttons[Buttons::OK]->set_position(QUIZ_OK_AT);

		title = Text(Text::Font::A15B, Text::Alignment::LEFT, Color::Name::WHITE, t);
		question = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, q);
		clue = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, c);
		question_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Question:");
		// v83's label starts with a tab (StringPool 0xf4d), how wide is not known.
		clue_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Clue:");
		enter_answer = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "[Enter Answer]");
		hint = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BRIGHTRED,
					"Enter an answer within the time limit.");
	}

	void UIQuiz::draw(float alpha) const
	{
		UIQuizWindow::draw(alpha);

		// v83 draws each on one line of the window's own layer, so long text is cut off at its edges.
		Range<int16_t> window(position.x(), position.x() + dimension.x());
		Point<int16_t> raise(0, TEXT_RAISE);

		title.draw_between(position + Point<int16_t>((dimension.x() - title.width()) >> 1, TITLE_Y - TITLE_RAISE),
						   window);

		if (!question.get_text().empty())
		{
			question_label.draw(position + QUESTION_LABEL_AT - raise);
			question.draw_between(position + QUESTION_AT - raise, window);
		}

		if (!clue.get_text().empty())
		{
			clue_label.draw(position + CLUE_LABEL_AT - raise);
			clue.draw_between(position + CLUE_AT - raise, window);
		}

		enter_answer.draw(position + ENTER_ANSWER_AT - raise);
		hint.draw(position + HINT_AT - raise);

		UIElement::draw_buttons(alpha);
		field.draw(position);
	}

	UIElement::Type UIQuiz::get_type() const
	{
		return TYPE;
	}

	Button::State UIQuiz::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			submit();

		return Button::State::NORMAL;
	}

	// An answer of nothing but blanks does nothing; one of the wrong length gets a notice (7eae53).
	void UIQuiz::submit()
	{
		std::string text = field.get_text();

		if (npc_input::trim(text).empty())
			return;

		std::string problem = quiz::length_problem(text, min_letters, max_letters);

		if (!problem.empty())
		{
			if (field.get_state() == Textfield::State::FOCUSED)
				UI::get().remove_textfield();

			UI::get().emplace<UIOk>(problem, [](bool)
			{
				if (auto quiz = UI::get().get_element<UIQuiz>())
					quiz->focus();
			});

			return;
		}

		answer(text);
	}

	UISpeedQuiz::UISpeedQuiz(int32_t subject_type, int32_t subject_id, int32_t p, int32_t n, int32_t seconds)
			: UIQuizWindow(NxFiles::ui_classic["UIWindow.img"]["SpeedQuiz"], 6, seconds,
						   box_at(SPEED_BOX_AT, SPEED_BOX_WIDTH)), picture_at(PICTURE_AT)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["SpeedQuiz"];

		buttons[Buttons::OK]->set_position(SPEED_OK_AT);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(src["BtNext"], NEXT_AT);
		buttons[Buttons::GIVE_UP] = std::make_unique<MapleButton>(src["BtGiveup"], GIVE_UP_AT);

		// An NPC, a monster, or otherwise an item (7ec595).
		std::string image = string_format::extend_id(subject_id, 7) + ".img";

		if (subject_type == 0)
			picture = nl::nx::npc[image]["stand"]["0"];
		else if (subject_type == 1)
			picture = nl::nx::mob[image]["stand"]["0"];
		else
			picture = ItemData::get(subject_id).get_icon(false);

		if (subject_type == 2)
			picture_at.shift_x(-ITEM_SHIFT);

		points = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, format("%2d pts", p));
		number = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, format("%2d", n));
		timer = nl::nx::sound["MiniGame.img"]["Timer"];
	}

	void UISpeedQuiz::draw(float alpha) const
	{
		UIQuizWindow::draw(alpha);

		Point<int16_t> raise(0, TEXT_RAISE);

		points.draw(position + POINTS_AT - raise);
		number.draw(position + NUMBER_AT - raise);
		picture.draw(position + picture_at);

		UIElement::draw_buttons(alpha);
		field.draw(position);
	}

	UIElement::Type UISpeedQuiz::get_type() const
	{
		return TYPE;
	}

	// Next and Give up answer at once (7ec46b).
	Button::State UISpeedQuiz::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				submit();
				break;
			case Buttons::NEXT:
				answer("");
				break;
			case Buttons::GIVE_UP:
				answer("__GIVEUP__");
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	// MiniGame.img's Timer each second of the last two minutes, louder as the time runs out (7ec825).
	void UISpeedQuiz::tick(int32_t seconds)
	{
		if (seconds <= 120)
			timer.play(static_cast<uint8_t>(std::clamp(quiz::tick_volume(seconds, get_total()), 0, 100)));
	}

	// An answer under two letters, not counting blanks at either end, does nothing (7ec497).
	void UISpeedQuiz::submit()
	{
		std::string text = field.get_text();

		if (npc_input::trim(text).size() < 2)
			return;

		answer(text);
	}

	bool taking_quiz()
	{
		auto quiz = UI::get().get_element<UIQuiz>();
		auto speed_quiz = UI::get().get_element<UISpeedQuiz>();

		return (quiz && quiz->is_active()) || (speed_quiz && speed_quiz->is_active());
	}
}
