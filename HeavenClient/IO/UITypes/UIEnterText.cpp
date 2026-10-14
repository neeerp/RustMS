#include "UIEnterText.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include <nlnx/nx.hpp>

namespace ms
{
	// Laid out like UIEnterNumber.
	UIEnterText::UIEnterText(std::string message, std::function<void(const std::string&)> th, size_t limit)
			: UINotice(message, NoticeType::ENTERNUMBER), texthandler(th)
	{
		int16_t belowtext = box2offset(true) - 21;
		int16_t pos_y = belowtext + 35;

		nl::node src = nl::nx::ui["Basic.img"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOK4"], 156, pos_y);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel4"], 198, pos_y);

		textfield = Textfield(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK,
							  Rectangle<int16_t>(24, 232, belowtext, belowtext + 20), limit);

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
					deactivate();
				}
		);

		textfield.set_state(Textfield::State::FOCUSED);
	}

	void UIEnterText::draw(float alpha) const
	{
		UINotice::draw(true);
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
		if (textfield.get_state() == Textfield::State::NORMAL)
		{
			Cursor::State tstate = textfield.send_cursor(cursorpos, clicked);

			if (tstate != Cursor::State::IDLE)
				return tstate;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIEnterText::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (keycode == KeyAction::Id::RETURN)
			submit();
		else if (escape)
			deactivate();
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
				deactivate();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIEnterText::submit()
	{
		// The field and the window may both see the same Enter press.
		if (!is_active())
			return;

		std::string text = textfield.get_text();

		if (!text.empty())
			texthandler(text);

		deactivate();
	}
}
