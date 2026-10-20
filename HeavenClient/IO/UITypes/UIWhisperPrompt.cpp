#include "UIWhisperPrompt.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// A compact type 7 CUtilDlgEx is 99 pixels taller than its text
		// (FUN_009acd54), whose lines wrap at 225 (FUN_009a3d07) and are 14
		// apart within a margin of 2 (FUN_009a2421).
		constexpr int16_t BASE_HEIGHT = 99;
		constexpr uint16_t WRAP = 225;
		constexpr int16_t LINE = 14;
		constexpr int16_t MARGIN = 2;
		// The prompt's first line, by the top of its text (FUN_009a4159).
		constexpr Point<int16_t> PROMPT = Point<int16_t>(20, 24);
		// The combo box and its edit (FUN_009a8e0c), and the buttons, from
		// the bottom.
		constexpr Point<int16_t> COMBO = Point<int16_t>(19, 73);
		constexpr Point<int16_t> COMBO_SIZE = Point<int16_t>(230, 18);
		constexpr Point<int16_t> EDIT = Point<int16_t>(24, 71);
		constexpr Point<int16_t> EDIT_SIZE = Point<int16_t>(206, 14);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(158, 30);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(208, 30);
		// The strip above the text moves it (FUN_009ac51b).
		constexpr int16_t DRAG_HEIGHT = 22;

		// A line of Arial 12.
		int16_t line_height()
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return line;
		}

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			return y + 11 - line_height();
		}

		// Every CUtilDlgEx opens around where the last one was, first the
		// middle of the screen (FUN_009a3e38, FUN_009ab4cd).
		Point<int16_t> &centre()
		{
			static Point<int16_t> centre = Point<int16_t>(Constants::Constants::get().get_viewwidth() / 2,
														  Constants::Constants::get().get_viewheight() / 2);

			return centre;
		}

		Rectangle<int16_t> from_bottom(Point<int16_t> pos, Point<int16_t> size, int16_t height)
		{
			int16_t top = height - pos.y();

			return Rectangle<int16_t>(pos.x(), pos.x() + size.x(), top, top + size.y());
		}
	}

	UIWhisperPrompt::UIWhisperPrompt(std::vector<std::string> history, OkHandler oh)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), okhandler(oh)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		top = basic["Notice3"]["t"];
		middle = basic["Notice3"]["c"];
		bottom = basic["Notice4"]["s"];

		int16_t line = line_height();
		prompt = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
					  "Please enter the person you want to whisper to.", WRAP, true, line - LINE);
		int16_t lines = 1 + (prompt.height() - line) / LINE;
		int16_t height = BASE_HEIGHT + 2 * MARGIN + LINE * lines;
		dimension = Point<int16_t>(top.width(), height);

		int16_t x = std::clamp<int16_t>(centre().x() - dimension.x() / 2, 0,
										Constants::Constants::get().get_viewwidth() - dimension.x());
		int16_t y = std::clamp<int16_t>(centre().y() - dimension.y() / 2, 0,
										Constants::Constants::get().get_viewheight() - dimension.y());
		position = Point<int16_t>(x, y);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"],
															 Point<int16_t>(OK_POS.x(), height - OK_POS.y()));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"],
																 Point<int16_t>(CANCEL_POS.x(), height - CANCEL_POS.y()));

		// Newest first, with the first copied into the edit (FUN_009ac844).
		combo = ComboEdit(from_bottom(COMBO, COMBO_SIZE, height), from_bottom(EDIT, EDIT_SIZE, height), 0,
						  std::move(history), true);
		Textfield &edit = combo.field();

		edit.set_enter_callback(
				[&](std::string)
				{
					if (!combo.send_key(KeyAction::Id::RETURN))
						ok();
				}
		);

		edit.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		for (KeyAction::Id key : { KeyAction::Id::UP, KeyAction::Id::DOWN })
			edit.set_key_callback(
					key,
					[&, key]()
					{
						combo.send_key(key);
					}
			);

		edit.set_state(Textfield::State::FOCUSED);
	}

	void UIWhisperPrompt::draw(float alpha) const
	{
		top.draw(position);

		int16_t bottom_y = dimension.y() - bottom.height();

		for (int16_t y = top.height(); y < bottom_y; y += middle.height())
			middle.draw(position + Point<int16_t>(0, y));

		bottom.draw(position + Point<int16_t>(0, bottom_y));

		prompt.draw(position + Point<int16_t>(PROMPT.x(), text_y(PROMPT.y())));

		combo.draw(position);

		UIElement::draw(alpha);

		combo.draw_list(position);
	}

	void UIWhisperPrompt::update()
	{
		UIElement::update();

		combo.update(position);

		// The edit keeps the keys (FUN_004c6786).
		Textfield &edit = combo.field();

		if (edit.get_state() != Textfield::State::FOCUSED)
			edit.set_state(Textfield::State::FOCUSED);
	}

	Cursor::State UIWhisperPrompt::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - cursoroffset;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		if (std::optional<Cursor::State> state = combo.send_cursor(clicked, cursorpos - position))
			return *state;

		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);
		Point<int16_t> pos = cursorpos - position;

		if (clicked && !is_holding_button() && pos.x() >= 0 && pos.x() < dimension.x() && pos.y() >= 0
			&& pos.y() < DRAG_HEIGHT)
		{
			dragged = true;
			cursoroffset = pos;
		}

		return state;
	}

	void UIWhisperPrompt::rightclick(Point<int16_t> cursorpos)
	{
		combo.rightclick(cursorpos - position);
	}

	void UIWhisperPrompt::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed || (combo.is_open() && combo.send_key(keycode)))
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			ok();
	}

	UIElement::Type UIWhisperPrompt::get_type() const
	{
		return TYPE;
	}

	Button::State UIWhisperPrompt::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIWhisperPrompt::ok()
	{
		std::string name = combo.field().get_text();
		std::vector<std::string> history = combo.get_items();

		close();
		okhandler(name, history);
	}

	void UIWhisperPrompt::close()
	{
		centre() = position + dimension / 2;
		combo.field().set_state(Textfield::State::DISABLED);
		deactivate();
	}
}
