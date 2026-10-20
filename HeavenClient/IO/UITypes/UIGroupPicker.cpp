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
		// The prompt's first line, by the top of its text (FUN_0099311b).
		constexpr Point<int16_t> PROMPT = Point<int16_t>(20, 29);
		// The combo box (FUN_00991899) and its edit after the left cap
		// (FUN_004c45c2).
		constexpr Rectangle<int16_t> COMBO = Rectangle<int16_t>(19, 245, 45, 63);
		constexpr Rectangle<int16_t> EDIT = Rectangle<int16_t>(24, 226, 47, 61);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(155, 86);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(205, 86);
		// A buddy's group is 16 characters at most (GW_Friend).
		constexpr size_t GROUP_LIMIT = 16;

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}
	}

	UIGroupPicker::UIGroupPicker(std::vector<std::string> choices, std::function<void(const std::string &)> oh)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), okhandler(oh)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node frame = basic["YesNo3"];
		top = frame["t"];
		middle = frame["c"];
		bottom = frame["s"];

		dimension = Point<int16_t>(top.width(), top.height() + MIDDLE_ROWS * middle.height() + bottom.height());
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		prompt = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Please select a group to whisper.");

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_POS);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_POS);
		// The keys start on Cancel (FUN_00991899), and so leave the chat
		// field Tab may have opened this from.
		buttons[Buttons::CANCEL]->set_focused(true);
		UI::get().remove_textfield();

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

		prompt.draw(position + Point<int16_t>(PROMPT.x(), text_y(PROMPT.y())));

		combo.draw(position);

		UIElement::draw(alpha);

		combo.draw_list(position);
	}

	void UIGroupPicker::update()
	{
		UIElement::update();

		combo.update(position);
	}

	Cursor::State UIGroupPicker::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (std::optional<Cursor::State> state = combo.send_cursor(clicked, cursorpos - position))
		{
			if (combo.field().get_state() == Textfield::State::FOCUSED)
				buttons[Buttons::CANCEL]->set_focused(false);

			return *state;
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

		close();
		okhandler(group);
	}

	void UIGroupPicker::close()
	{
		combo.field().set_state(Textfield::State::DISABLED);
		deactivate();
	}
}
