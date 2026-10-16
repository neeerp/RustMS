//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UINotice.h"

#include "EnterNumber.h"

#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Audio/Audio.h"

#include "../../Util/NxFiles.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// The pieces of the stretchable boxes: t holds the title strip, the top of the bolted plate and the top
		// edge of the text frame, c repeats as the inside of the text frame, and s holds its bottom edge, the
		// button frames and the bottom of the plate.
		constexpr int16_t TEXT_TOP = 38;
		constexpr int16_t TEXT_PADDING = 6;
		constexpr int16_t TEXT_WIDTH = 220;
		constexpr int16_t TEXT_LEFT = 20;
		constexpr int16_t MIN_ROWS = 2;
		constexpr int16_t FIELD_LEFT = 20;
		constexpr int16_t FIELD_RIGHT = 238;
		constexpr int16_t FIELD_HEIGHT = 18;
		constexpr int16_t FIELD_GAP = 4;

		// In s: the inside of the button frames, which fit the 65x24 buttons.
		constexpr int16_t BUTTON_TOP = 10;
		constexpr int16_t ONE_BUTTON_X = 97;
		constexpr int16_t LEFT_BUTTON_X = 53;
		constexpr int16_t RIGHT_BUTTON_X = 141;
	}

	UINotice::UINotice(std::string message, NoticeType t, Text::Alignment a)
			: UIDragElement<PosNOTICE>(), type(t), alignment(a)
	{
		nl::node src = NxFiles::ui_classic["Basic.img"][type == NoticeType::OK ? "Notice2" : "YesNo2"];

		top = src["t"];
		center = src["c"];
		bottom = src["s"];

		question = Text(Text::Font::A12M, alignment, Color::Name::BLACK, message, TEXT_WIDTH);

		int16_t needed = question.height() + 2 * TEXT_PADDING;

		if (type == NoticeType::ENTERNUMBER)
			needed += FIELD_GAP + FIELD_HEIGHT;

		// The text frame is 4 pixels taller than its rows: two in t and two in s.
		int16_t row_height = center.height();
		rows = std::max<int16_t>(MIN_ROWS, (needed - 4 + row_height - 1) / row_height);

		dimension = Point<int16_t>(top.width(), top.height() + rows * row_height + bottom.height());
		position = Point<int16_t>(position.x() - dimension.x() / 2, position.y() - dimension.y() / 2);
		dragarea = Point<int16_t>(dimension.x(), 19);

		if (type != NoticeType::ENTERNUMBER)
			Sound(Sound::Name::DLGNOTICE).play();
	}

	UINotice::UINotice(std::string message, NoticeType t) : UINotice(message, t, Text::Alignment::CENTER)
	{}

	void UINotice::draw() const
	{
		Point<int16_t> start = position;

		top.draw(start);
		start.shift_y(top.height());

		for (int16_t i = 0; i < rows; i++)
		{
			center.draw(start);
			start.shift_y(center.height());
		}

		bottom.draw(start);

		int16_t text_y = TEXT_TOP + TEXT_PADDING;

		if (type != NoticeType::ENTERNUMBER)
		{
			// The glyphs sit a few pixels below the top of their line, so the line is centred a little high.
			int16_t area = rows * center.height() + 4;
			text_y = TEXT_TOP + (area - question.height()) / 2 - 3;
		}

		int16_t text_x = alignment == Text::Alignment::LEFT ? TEXT_LEFT : dimension.x() / 2;

		question.draw(position + Point<int16_t>(text_x, text_y));
	}

	int16_t UINotice::buttons_y() const
	{
		return top.height() + rows * center.height() + BUTTON_TOP;
	}

	int16_t UINotice::field_y() const
	{
		return TEXT_TOP + TEXT_PADDING + question.height() + FIELD_GAP;
	}

	UIYesNo::UIYesNo(std::string message, std::function<void(bool yes)> yh, Text::Alignment alignment,
					 uint8_t a) : UINotice(message, NoticeType::YESNO, alignment)
	{
		yesnohandler = yh;
		asks = a;

		nl::node src = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::YES] = std::make_unique<MapleButton>(src["BtYes"], Point<int16_t>(LEFT_BUTTON_X, buttons_y()));
		buttons[Buttons::NO] = std::make_unique<MapleButton>(src["BtNo"], Point<int16_t>(RIGHT_BUTTON_X, buttons_y()));
	}

	UIYesNo::UIYesNo(std::string message, std::function<void(bool yes)> yesnohandler) : UIYesNo(message, yesnohandler,
																								Text::Alignment::CENTER)
	{}

	void UIYesNo::draw(float alpha) const
	{
		UINotice::draw();
		UIElement::draw(alpha);
	}

	void UIYesNo::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (keycode == KeyAction::Id::RETURN)
			answer(true);
		else if (escape)
			answer(false);
	}

	UIElement::Type UIYesNo::get_type() const
	{
		return TYPE;
	}

	Button::State UIYesNo::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::YES:
				if (!answer(true))
					return Button::State::NORMAL;
				break;
			case Buttons::NO:
				answer(false);
				break;
		}

		return Button::State::PRESSED;
	}

	bool UIYesNo::answer(bool yes)
	{
		if (yes && asks > 1)
		{
			asks--;

			return false;
		}

		deactivate();
		yesnohandler(yes);

		return true;
	}

	UIEnterNumber::UIEnterNumber(std::string message, std::function<void(int32_t)> nh, int32_t m, int32_t quantity)
			: UINotice(message, NoticeType::ENTERNUMBER)
	{
		numhandler = nh;
		max = m;

		nl::node src = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOK"], LEFT_BUTTON_X, buttons_y());
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], RIGHT_BUTTON_X, buttons_y());

		int16_t field_width = FIELD_RIGHT - FIELD_LEFT;
		fieldborder = ColorBox(field_width, FIELD_HEIGHT, Color::Name::DUSTYGRAY, 1.0f);
		fieldfill = ColorBox(field_width - 2, FIELD_HEIGHT - 2, Color::Name::WHITE, 1.0f);

		// Text draws below its anchor, so the bounds sit a little above the box.
		int16_t top = field_y();
		numfield = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
							 Rectangle<int16_t>(FIELD_LEFT + 4, FIELD_RIGHT - 4, top - 5, top + FIELD_HEIGHT - 5), 10);
		numfield.change_text(std::to_string(quantity));

		numfield.set_enter_callback(
				[&](std::string numstr)
				{
					handlestring(numstr);
				}
		);

		numfield.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					deactivate();
				}
		);

		numfield.set_state(Textfield::State::FOCUSED);
	}

	void UIEnterNumber::draw(float alpha) const
	{
		UINotice::draw();
		UIElement::draw(alpha);

		Point<int16_t> field = position + Point<int16_t>(FIELD_LEFT, field_y());
		fieldborder.draw(field);
		fieldfill.draw(field + Point<int16_t>(1, 1));

		numfield.draw(position);
	}

	void UIEnterNumber::update()
	{
		UIElement::update();

		numfield.update(position);
	}

	Cursor::State UIEnterNumber::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (numfield.get_state() == Textfield::State::NORMAL)
		{
			Cursor::State nstate = numfield.send_cursor(cursorpos, clicked);

			if (nstate != Cursor::State::IDLE)
				return nstate;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIEnterNumber::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (keycode == KeyAction::Id::RETURN)
		{
			handlestring(numfield.get_text());
		} else if (escape)
		{
			deactivate();
		}
	}

	UIElement::Type UIEnterNumber::get_type() const
	{
		return TYPE;
	}

	Button::State UIEnterNumber::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				handlestring(numfield.get_text());
				break;
			case Buttons::CANCEL:
				deactivate();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIEnterNumber::handlestring(std::string numstr)
	{
		auto okhandler = [&](bool)
		{
			numfield.set_state(Textfield::State::FOCUSED);
			buttons[Buttons::OK]->set_state(Button::State::NORMAL);
		};

		enter_number::Checked checked = enter_number::check(numstr, max);
		std::string problem;

		switch (checked.problem)
		{
			case enter_number::Problem::NONE:
				numhandler(checked.number);
				deactivate();
				buttons[Buttons::OK]->set_state(Button::State::NORMAL);
				return;
			case enter_number::Problem::NOT_A_NUMBER:
				problem = "Only numbers are allowed.";
				break;
			case enter_number::Problem::TOO_LOW:
				problem = "You may only enter a number equal to or higher than 1.";
				break;
			case enter_number::Problem::TOO_HIGH:
				problem = "You may only enter a number equal to or lower than " + std::to_string(max) + ".";
				break;
		}

		numfield.set_state(Textfield::State::DISABLED);
		UI::get().emplace<UIOk>(problem, okhandler);
	}

	UIEnterText::UIEnterText(std::string message, std::function<void(const std::string &)> th, size_t limit)
			: UINotice(message, NoticeType::ENTERNUMBER), texthandler(th)
	{
		nl::node src = NxFiles::ui_classic["Basic.img"];
		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOK"], LEFT_BUTTON_X, buttons_y());
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], RIGHT_BUTTON_X, buttons_y());
		int16_t top = field_y();
		textfield = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
			Rectangle<int16_t>(FIELD_LEFT + 4, FIELD_RIGHT - 4, top - 5, top + FIELD_HEIGHT - 5), limit);

		textfield.set_enter_callback(
				[&](std::string)
				{
					submit();
				}
		);

		textfield.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		textfield.set_state(Textfield::State::FOCUSED);
	}

	void UIEnterText::draw(float alpha) const
	{
		UINotice::draw();
		UIElement::draw(alpha);

		textfield.draw(position);
	}

	void UIEnterText::update()
	{
		UIElement::update();

		textfield.update(position);
	}

	Cursor::State UIEnterText::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State tstate = textfield.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIEnterText::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (keycode == KeyAction::Id::RETURN)
			submit();
		else if (escape)
			close();
	}

	UIElement::Type UIEnterText::get_type() const
	{
		return TYPE;
	}

	Button::State UIEnterText::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				submit();
				break;
			case Buttons::CANCEL:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIEnterText::submit()
	{
		std::string text = textfield.get_text();

		if (text.empty())
			return;

		close();
		texthandler(text);
	}

	void UIEnterText::close()
	{
		textfield.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	UIOk::UIOk(std::string message, std::function<void(bool ok)> oh) : UINotice(message, NoticeType::OK)
	{
		okhandler = oh;

		nl::node src = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOK"], ONE_BUTTON_X, buttons_y());
	}

	void UIOk::draw(float alpha) const
	{
		UINotice::draw();
		UIElement::draw(alpha);
	}

	void UIOk::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (keycode == KeyAction::Id::RETURN)
			{
				okhandler(true);
				deactivate();
			} else if (escape)
			{
				okhandler(false);
				deactivate();
			}
		}
	}

	UIElement::Type UIOk::get_type() const
	{
		return TYPE;
	}

	Button::State UIOk::button_pressed(uint16_t buttonid)
	{
		deactivate();

		switch (buttonid)
		{
			case Buttons::OK:
				okhandler(true);
				break;
		}

		return Button::State::NORMAL;
	}
}