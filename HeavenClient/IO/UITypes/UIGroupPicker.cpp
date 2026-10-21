#include "UIGroupPicker.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// CUtilDlg (FUN_00991803) stacks YesNo3's top, two middle rows for a
		// two-line prompt and its bottom.
		constexpr int16_t MIDDLE_ROWS = 2;
		// The prompt's first line, by the top of its text, and the next's
		// (FUN_0099311b).
		constexpr Point<int16_t> PROMPT = Point<int16_t>(20, 29);
		constexpr int16_t PROMPT_LINE = 14;
		// The combo box (FUN_00991899) and its edit after the left cap
		// (FUN_004c45c2).
		constexpr Rectangle<int16_t> COMBO = Rectangle<int16_t>(19, 245, 45, 63);
		constexpr Rectangle<int16_t> EDIT = Rectangle<int16_t>(24, 226, 47, 61);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(155, 86);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(205, 86);
		// A buddy's group is 16 characters at most (GW_Friend).
		constexpr size_t GROUP_LIMIT = 16;
		// Add Buddy's taller bottom (Notice4/s_line) has the name's white box
		// at (19, 75), 226 by 15, its text 2 in; HeavenClient draws a
		// Textfield's text 5 below its top. The buttons sit 29 lower.
		constexpr Rectangle<int16_t> NAME = Rectangle<int16_t>(21, 245, 70, 85);
		constexpr size_t NAME_LIMIT = 12;
		constexpr int16_t NAME_BUTTONS = 29;

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}

		std::string trimmed(const std::string &text)
		{
			const char *space = " \t\r\n";
			size_t first = text.find_first_not_of(space);

			if (first == std::string::npos)
				return "";

			return text.substr(first, text.find_last_not_of(space) - first + 1);
		}
	}

	UIGroupPicker::UIGroupPicker(std::vector<std::string> choices, std::function<void(const std::string &)> oh)
			: UIGroupPicker(false, std::move(choices))
	{
		okhandler = std::move(oh);
	}

	UIGroupPicker::UIGroupPicker(const std::string &text, std::vector<std::string> choices,
								 std::function<void(const std::string &, const std::string &)> oh)
			: UIGroupPicker(true, std::move(choices))
	{
		name_okhandler = std::move(oh);
		name.change_text(text);
	}

	UIGroupPicker::UIGroupPicker(bool an, std::vector<std::string> choices)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), asks_name(an)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node frame = basic["YesNo3"];
		top = frame["t"];
		middle = frame["c"];
		// The dialog's taller bottom when it has an edit (FUN_00991878).
		bottom = asks_name ? basic["Notice4"]["s_line"] : frame["s"];

		dimension = Point<int16_t>(top.width(), top.height() + MIDDLE_ROWS * middle.height() + bottom.height());
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		std::vector<std::string> lines = asks_name
				? std::vector<std::string>{ "Please enter the name of the character", "you'll add as a buddy." }
				: std::vector<std::string>{ "Please select a group to whisper." };

		for (const std::string &line : lines)
			prompt.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, line);

		Point<int16_t> shift = Point<int16_t>(0, asks_name ? NAME_BUTTONS : 0);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_POS + shift);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_POS + shift);

		combo = ComboEdit(COMBO, EDIT, GROUP_LIMIT, std::move(choices), false);
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

		if (asks_name)
		{
			// Add Buddy starts in the name (FUN_00992314).
			name = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NAME, NAME_LIMIT);
			name.set_enter_callback([&](std::string) { ok(); });
			name.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
			name.set_state(Textfield::State::FOCUSED);
		}
		else
		{
			// The keys start on Cancel (FUN_00991899), and so leave the chat
			// field Tab may have opened this from.
			buttons[Buttons::CANCEL]->set_focused(true);
			UI::get().remove_textfield();
		}

		Sound(Sound::Name::DLGNOTICE).play();
	}

	void UIGroupPicker::draw(float alpha) const
	{
		Point<int16_t> pos = position;
		top.draw(pos);
		pos.shift_y(top.height());

		for (int16_t i = 0; i < MIDDLE_ROWS; i++)
		{
			middle.draw(pos);
			pos.shift_y(middle.height());
		}

		bottom.draw(pos);

		for (size_t i = 0; i < prompt.size(); i++)
			prompt[i].draw(position + Point<int16_t>(PROMPT.x(), text_y(PROMPT.y() + PROMPT_LINE * i)));

		if (asks_name)
			name.draw(position);

		combo.draw(position);

		UIElement::draw(alpha);

		combo.draw_list(position);
	}

	void UIGroupPicker::update()
	{
		UIElement::update();

		combo.update(position);

		if (asks_name)
			name.update(position);
	}

	Cursor::State UIGroupPicker::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (std::optional<Cursor::State> state = combo.send_cursor(clicked, cursorpos - position))
		{
			if (combo.field().get_state() == Textfield::State::FOCUSED)
				buttons[Buttons::CANCEL]->set_focused(false);

			return *state;
		}

		if (asks_name)
		{
			if (Cursor::State state = name.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
				return state;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGroupPicker::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed || (combo.is_open() && combo.send_key(keycode)))
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			ok();
	}

	UIElement::Type UIGroupPicker::get_type() const
	{
		return TYPE;
	}

	Button::State UIGroupPicker::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIGroupPicker::ok()
	{
		std::string group = combo.field().get_text();
		std::string text = trimmed(name.get_text());

		close();

		if (asks_name)
			name_okhandler(text, group);
		else
			okhandler(group);
	}

	void UIGroupPicker::close()
	{
		combo.field().set_state(Textfield::State::DISABLED);

		if (asks_name)
			name.set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
