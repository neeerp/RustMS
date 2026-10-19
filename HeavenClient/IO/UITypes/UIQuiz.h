#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/Text.h"
#include "../../Timer.h"

#include <nlnx/node.hpp>

#include <cstdint>
#include <string>

namespace ms
{
	// What the quiz and the speed quiz share: a countdown, an answer box, and one answer sent when the player
	// gives it or the time runs out. Unlike the rest of an NPC's dialogue they are not modal, and Escape does
	// not close them.
	class UIQuizWindow : public UIElement
	{
	public:
		~UIQuizWindow() override;

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		// Close without answering, as the server asks.
		void hide();

		// Give the answer box the keyboard back.
		void focus();

	protected:
		UIQuizWindow(nl::node src, int8_t msgtype, int32_t seconds, Rectangle<int16_t> box);

		// Answer once and close.
		void answer(const std::string &text);

		// Called each second, and once more when the time is up.
		virtual void tick(int32_t seconds)
		{}

		// What the OK button or Enter does.
		virtual void submit() = 0;

		int32_t get_total() const;

		Textfield field;

	private:
		int8_t msgtype;
		ContinuousTimer::point opened;
		int32_t total;
		int32_t shown;
		bool answered;
		Texture digits[10];
		Texture comma;
	};

	// An NPC's quiz (dialogue type 5, v83 CQuizDlg: 7e9bc4, 7e9d54, 7eb16e): a title, a question and a clue
	// if given, and a box for the answer. OK or Enter answers when it is the right length.
	class UIQuiz : public UIQuizWindow
	{
	public:
		static constexpr Type TYPE = UIElement::Type::QUIZ;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIQuiz(const std::string &title, const std::string &question, const std::string &clue, int32_t min_letters,
			   int32_t max_letters, int32_t seconds);

		void draw(float alpha) const override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		void submit() override;

	private:
		Text title;
		Text question_label;
		Text question;
		Text clue_label;
		Text clue;
		Text enter_answer;
		Text hint;
		int32_t min_letters;
		int32_t max_letters;
	};

	// An NPC's speed quiz (dialogue type 6, v83 7eb516, 7eb7d5, 7ec7cb): an NPC, monster or item to name, the
	// points so far and a second number, likely the question's. OK or Enter answers with two or more letters,
	// NEXT with nothing, GIVE UP with "__GIVEUP__". The last two minutes tick.
	class UISpeedQuiz : public UIQuizWindow
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SPEEDQUIZ;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UISpeedQuiz(int32_t subject_type, int32_t subject_id, int32_t points, int32_t number, int32_t seconds);

		void draw(float alpha) const override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		void tick(int32_t seconds) override;

		void submit() override;

	private:
		Texture picture;
		Point<int16_t> picture_at;
		Text points;
		Text number;
		Sound timer;
	};

	// Whether the player is answering a quiz, which keeps them out of the Cash Shop and from changing channels
	// (v83 0xa04dca, 0xa06735).
	bool taking_quiz();
}
