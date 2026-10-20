#include "ComboEdit.h"

#include "../KeyAction.h"

#include "../../Constants.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Each row's text by its top (FUN_004c3a0d), in a pixel of border.
		constexpr int16_t ROW = 16;
		constexpr Point<int16_t> ROW_TEXT = Point<int16_t>(7, 3);
		constexpr Color LIST_BORDER = Color(0x999999FF);
		constexpr Color LIST_FILL = Color(0xEEEEEEFF);
		constexpr Color LIST_LIT = Color(0xA5A198FF);
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

	ComboEdit::ComboEdit(Rectangle<int16_t> b, Rectangle<int16_t> e, size_t limit, std::vector<std::string> i,
						 bool r) : box(b), edit_bounds(e), items(std::move(i)), removable(r)
	{
		nl::node combo = NxFiles::ui_classic["Basic.img"]["ComboBox"];
		const char *states[NUM_FACES] = { "normal", "mouseOver", "pressed", "disabled" };

		for (size_t face = 0; face < NUM_FACES; face++)
			for (size_t piece = 0; piece < 3; piece++)
				faces[face][piece] = combo[states[face]][std::to_string(piece)];

		for (const std::string &item : items)
		{
			rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, item);
			lit_rows.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, item);
		}

		edit = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						 Rectangle<int16_t>(e.left(), e.right(), text_y(e.top()), e.bottom()), limit);
		show_selected();
	}

	ComboEdit::ComboEdit() = default;

	void ComboEdit::draw(Point<int16_t> parentpos) const
	{
		// The left cap, as many middles as fit with a pixel to spare, and the
		// right cap after them (FUN_004c5fb9). With no items it is disabled
		// (FUN_004c6e0f).
		Face face = items.empty() ? Face::DISABLED
								  : pressed_time > 0 ? Face::PRESSED : hovered ? Face::MOUSEOVER : Face::NORMAL;
		const Texture *pieces = faces[face];
		Point<int16_t> piece = parentpos + box.get_left_top();
		pieces[0].draw(piece);
		piece.shift_x(pieces[0].width());

		int16_t mid_width = pieces[1].width();
		int16_t tiles = mid_width > 0 ? (box.width() - pieces[0].width() - pieces[2].width() - 1) / mid_width : 0;

		for (int16_t i = 0; i < tiles; i++)
		{
			pieces[1].draw(piece);
			piece.shift_x(mid_width);
		}

		pieces[2].draw(piece);

		edit.draw(parentpos);
	}

	void ComboEdit::draw_list(Point<int16_t> parentpos) const
	{
		if (!open)
			return;

		GraphicsGL &graphics = GraphicsGL::get();
		Rectangle<int16_t> list = list_bounds();
		Point<int16_t> origin = parentpos + list.get_left_top();
		int16_t width = list.width();
		int16_t height = list.height();

		graphics.drawrectangle(origin.x(), origin.y(), width - 1, height, LIST_BORDER.r(), LIST_BORDER.g(),
							   LIST_BORDER.b(), LIST_BORDER.a());
		graphics.drawrectangle(origin.x() + 1, origin.y() + 1, width - 3, height - 2, LIST_FILL.r(), LIST_FILL.g(),
							   LIST_FILL.b(), LIST_FILL.a());

		for (size_t i = 0; i < rows.size(); i++)
		{
			int16_t row_top = static_cast<int16_t>(ROW * i);
			bool lit = i == hover;

			if (lit)
				graphics.drawrectangle(origin.x() + 1, origin.y() + row_top + 1, width - 2, ROW - 1, LIST_LIT.r(),
									   LIST_LIT.g(), LIST_LIT.b(), LIST_LIT.a());

			(lit ? lit_rows : rows)[i].draw(origin + Point<int16_t>(ROW_TEXT.x(), text_y(row_top + ROW_TEXT.y())));
		}
	}

	void ComboEdit::update(Point<int16_t> parentpos)
	{
		edit.update(parentpos);

		pressed_time = pressed_time > Constants::TIMESTEP ? pressed_time - Constants::TIMESTEP : 0;
	}

	std::optional<Cursor::State> ComboEdit::send_cursor(bool clicked, Point<int16_t> pos)
	{
		bool pressed = clicked && !mouse_down;
		bool released = !clicked && mouse_down;
		mouse_down = clicked;

		bool on_box = box.contains(pos) && !edit_bounds.contains(pos);
		hovered = on_box;

		if (open)
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
				open = false;

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
				toggle_list();
			}

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		if (edit_bounds.contains(pos))
		{
			if (pressed)
				edit.set_state(Textfield::State::FOCUSED);

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return std::nullopt;
	}

	bool ComboEdit::rightclick(Point<int16_t> pos)
	{
		if (!open || !removable)
			return false;

		std::optional<size_t> row = row_at(pos);

		if (row)
			remove(*row);

		return row.has_value();
	}

	bool ComboEdit::send_key(int32_t keycode)
	{
		// Down from the edit opens and closes the dropdown (FUN_004c7343);
		// an open dropdown takes Up, Down and Enter (FUN_004c405e).
		if (!open)
		{
			if (keycode != KeyAction::Id::DOWN)
				return false;

			toggle_list();

			return true;
		}

		switch (keycode)
		{
			case KeyAction::Id::UP:
				if (hover > 0)
					hover--;

				return true;
			case KeyAction::Id::DOWN:
				hover = std::min(hover + 1, items.size() - 1);

				return true;
			case KeyAction::Id::RETURN:
				pick(hover);

				return true;
			default:
				return false;
		}
	}

	Textfield &ComboEdit::field()
	{
		return edit;
	}

	const std::vector<std::string> &ComboEdit::get_items() const
	{
		return items;
	}

	bool ComboEdit::is_open() const
	{
		return open;
	}

	void ComboEdit::toggle_list()
	{
		// It opens on the selected item, and not at all with none
		// (FUN_004c7068).
		if (open || items.empty())
		{
			open = false;

			return;
		}

		open = true;
		hover = selected;
	}

	void ComboEdit::pick(size_t row)
	{
		// FUN_004c738b closes the dropdown and selects the row.
		open = false;
		selected = row;
		show_selected();
	}

	void ComboEdit::remove(size_t row)
	{
		// FUN_004c73c0 selects the first item, drops the row and reopens the
		// dropdown while items are left.
		selected = 0;
		items.erase(items.begin() + row);
		rows.erase(rows.begin() + row);
		lit_rows.erase(lit_rows.begin() + row);
		open = false;
		toggle_list();
		show_selected();
	}

	void ComboEdit::show_selected()
	{
		if (items.empty() || shown == selected)
			return;

		// The item comes in selected, for typing to replace (FUN_004c67ad).
		edit.change_text(items[selected]);
		edit.select_all();
		shown = selected;
	}

	std::optional<size_t> ComboEdit::row_at(Point<int16_t> pos) const
	{
		Rectangle<int16_t> list = list_bounds();

		if (!list.contains(pos))
			return std::nullopt;

		size_t row = static_cast<size_t>((pos.y() - list.top()) / ROW);

		return row < items.size() ? std::optional<size_t>(row) : std::nullopt;
	}

	Rectangle<int16_t> ComboEdit::list_bounds() const
	{
		int16_t height = static_cast<int16_t>(ROW * items.size() + 1);

		return Rectangle<int16_t>(box.left(), box.right(), box.bottom(), box.bottom() + height);
	}
}
