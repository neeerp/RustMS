#include "UIGroupPicker.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Graphics/GraphicsGL.h"
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
		// The combo box (FUN_00991899), its edit after the left cap
		// (FUN_004c45c2), and its list: 16 pixel rows below it, in a pixel
		// of border, with each row's text by its top (FUN_004c3a0d).
		constexpr Rectangle<int16_t> COMBO = Rectangle<int16_t>(19, 245, 45, 63);
		constexpr Rectangle<int16_t> EDIT = Rectangle<int16_t>(24, 226, 47, 61);
		constexpr int16_t ROW = 16;
		constexpr Point<int16_t> ROW_TEXT = Point<int16_t>(7, 3);
		constexpr Color LIST_BORDER = Color(0x999999FF);
		constexpr Color LIST_FILL = Color(0xEEEEEEFF);
		constexpr Color LIST_LIT = Color(0xA5A198FF);
		constexpr Point<int16_t> OK_POS = Point<int16_t>(155, 86);
		constexpr Point<int16_t> CANCEL_POS = Point<int16_t>(205, 86);
		// A buddy's group is 16 characters at most (GW_Friend).
		constexpr size_t GROUP_LIMIT = 16;
		// How long a click shows the box pressed (FUN_004c6faf).
		constexpr uint16_t PRESS_TIME = 300;

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}
	}

	UIGroupPicker::UIGroupPicker(std::vector<std::string> c, std::function<void(const std::string &)> oh)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), okhandler(oh), choices(std::move(c))
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node frame = basic["YesNo3"];
		top = frame["t"];
		middle = frame["c"];
		bottom = frame["s"];

		const char *states[NUM_FACES] = { "normal", "mouseOver", "pressed" };

		for (size_t i = 0; i < NUM_FACES; i++)
			for (size_t piece = 0; piece < 3; piece++)
				faces[i][piece] = basic["ComboBox"][states[i]][std::to_string(piece)];

		dimension = Point<int16_t>(top.width(), top.height() + MIDDLE_ROWS * middle.height() + bottom.height());
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		prompt = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Please select a group to whisper.");

		for (const std::string &choice : choices)
		{
			rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, choice);
			lit_rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, choice);
		}

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_POS);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_POS);
		// The keys start on Cancel (FUN_00991899), and so leave the chat
		// field Tab may have opened this from.
		buttons[Buttons::CANCEL]->set_focused(true);
		UI::get().remove_textfield();

		edit = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						 Rectangle<int16_t>(EDIT.left(), EDIT.right(), text_y(EDIT.top()), EDIT.bottom()), GROUP_LIMIT);
		edit.change_text(choices.empty() ? "" : choices[selected]);

		edit.set_enter_callback(
				[&](std::string)
				{
					if (!list_key(KeyAction::Id::RETURN))
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
						list_key(key);
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

		// The left cap, as many middles as fit with a pixel to spare, and the
		// right cap after them (FUN_004c5fb9).
		Face face = pressed_time > 0 ? Face::PRESSED : combo_hovered ? Face::MOUSEOVER : Face::NORMAL;
		const Texture *pieces = faces[face];
		Point<int16_t> piece = position + COMBO.get_left_top();
		pieces[0].draw(piece);
		piece.shift_x(pieces[0].width());

		int16_t mid_width = pieces[1].width();
		int16_t tiles = mid_width > 0 ? (COMBO.width() - pieces[0].width() - pieces[2].width() - 1) / mid_width : 0;

		for (int16_t i = 0; i < tiles; i++)
		{
			pieces[1].draw(piece);
			piece.shift_x(mid_width);
		}

		pieces[2].draw(piece);

		edit.draw(position);

		UIElement::draw(alpha);

		if (list_open)
		{
			GraphicsGL &graphics = GraphicsGL::get();
			Rectangle<int16_t> list = list_bounds();
			Point<int16_t> origin = position + list.get_left_top();
			int16_t width = list.width();
			int16_t height = list.height();

			graphics.drawrectangle(origin.x(), origin.y(), width - 1, height, LIST_BORDER.r(), LIST_BORDER.g(),
								   LIST_BORDER.b(), LIST_BORDER.a());
			graphics.drawrectangle(origin.x() + 1, origin.y() + 1, width - 3, height - 2, LIST_FILL.r(),
								   LIST_FILL.g(), LIST_FILL.b(), LIST_FILL.a());

			for (size_t i = 0; i < rows.size(); i++)
			{
				int16_t row_top = static_cast<int16_t>(ROW * i);
				bool lit = i == hover;

				if (lit)
					graphics.drawrectangle(origin.x() + 1, origin.y() + row_top + 1, width - 2, ROW - 1, LIST_LIT.r(),
										   LIST_LIT.g(), LIST_LIT.b(), LIST_LIT.a());

				(lit ? lit_rows : rows)[i].draw(
						origin + Point<int16_t>(ROW_TEXT.x(), text_y(row_top + ROW_TEXT.y())));
			}
		}
	}

	void UIGroupPicker::update()
	{
		UIElement::update();

		edit.update(position);

		pressed_time = pressed_time > Constants::TIMESTEP ? pressed_time - Constants::TIMESTEP : 0;
	}

	Cursor::State UIGroupPicker::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool pressed = clicked && !mouse_down;
		bool released = !clicked && mouse_down;
		mouse_down = clicked;

		Point<int16_t> pos = cursorpos - position;
		bool on_box = COMBO.contains(pos) && !EDIT.contains(pos);
		combo_hovered = on_box;

		if (list_open)
		{
			// The row under the cursor lights up and is picked when the
			// button comes up on it (FUN_004c4121).
			if (std::optional<size_t> row = row_at(pos))
			{
				hover = *row;

				if (released)
					pick(*row);

				return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
			}

			if (pressed)
			{
				list_open = false;

				// A click on the box only closes it.
				if (on_box)
				{
					pressed_time = PRESS_TIME;

					return Cursor::State::CLICKING;
				}
			}
		}

		if (on_box)
		{
			if (pressed)
			{
				pressed_time = PRESS_TIME;
				open_list();
			}

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		if (EDIT.contains(pos))
		{
			if (pressed)
			{
				edit.set_state(Textfield::State::FOCUSED);
				buttons[Buttons::CANCEL]->set_focused(false);
			}

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGroupPicker::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed || list_key(keycode))
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
		std::string group = edit.get_text();

		close();
		okhandler(group);
	}

	void UIGroupPicker::close()
	{
		edit.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	void UIGroupPicker::open_list()
	{
		// It opens on the current choice, and not at all with none.
		if (choices.empty())
			return;

		list_open = true;
		hover = selected;
	}

	void UIGroupPicker::pick(size_t row)
	{
		// Picking a row fills the edit and leaves the dialog open
		// (FUN_004c738b).
		list_open = false;
		selected = row;
		edit.change_text(choices[row]);
	}

	void UIGroupPicker::move_hover(bool down)
	{
		if (down)
			hover = std::min(hover + 1, choices.size() - 1);
		else if (hover > 0)
			hover--;
	}

	bool UIGroupPicker::list_key(int32_t keycode)
	{
		if (!list_open)
			return false;

		// Up and Down move along the list and Enter picks the row
		// (FUN_004c405e).
		switch (keycode)
		{
			case KeyAction::Id::UP:
				move_hover(false);
				return true;
			case KeyAction::Id::DOWN:
				move_hover(true);
				return true;
			case KeyAction::Id::RETURN:
				pick(hover);
				return true;
			default:
				return false;
		}
	}

	std::optional<size_t> UIGroupPicker::row_at(Point<int16_t> pos) const
	{
		Rectangle<int16_t> list = list_bounds();

		if (!list.contains(pos))
			return std::nullopt;

		size_t row = static_cast<size_t>((pos.y() - list.top()) / ROW);

		return row < choices.size() ? std::optional<size_t>(row) : std::nullopt;
	}

	Rectangle<int16_t> UIGroupPicker::list_bounds() const
	{
		int16_t height = static_cast<int16_t>(ROW * choices.size() + 1);

		return Rectangle<int16_t>(COMBO.left(), COMBO.right(), COMBO.bottom(), COMBO.bottom() + height);
	}
}
