#pragma once

#include "Textfield.h"

#include "../Cursor.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// v83's editable combo box (CCtrlComboBox, FUN_004c4259): Basic.img's
	// ComboBox art around an edit, with a dropdown of the items in 16 pixel
	// rows below it. Picking a row copies it into the edit. The owner sets
	// the edit's callbacks, since a focused Textfield must not move.
	class ComboEdit
	{
	public:
		// The box and edit in the owner's space; removable items go with a
		// right click on their row.
		ComboEdit(Rectangle<int16_t> box, Rectangle<int16_t> edit, size_t limit, std::vector<std::string> items,
				  bool removable);
		ComboEdit();

		void draw(Point<int16_t> parentpos) const;
		// The dropdown, drawn last to cover what is below the box.
		void draw_list(Point<int16_t> parentpos) const;
		void update(Point<int16_t> parentpos);

		// Takes the cursor at pos in the owner's space when it is on the
		// combo or the dropdown is open.
		std::optional<Cursor::State> send_cursor(bool clicked, Point<int16_t> pos);
		// A right click at pos removes the row under it, if allowed.
		bool rightclick(Point<int16_t> pos);
		// Down opens and closes the dropdown; while it is open, Up and Down
		// move along it and Enter picks.
		bool send_key(int32_t keycode);

		Textfield &field();
		const std::vector<std::string> &get_items() const;
		bool is_open() const;

	private:
		void toggle_list();
		void pick(size_t row);
		void remove(size_t row);
		void show_selected();
		std::optional<size_t> row_at(Point<int16_t> pos) const;
		Rectangle<int16_t> list_bounds() const;

		enum Face : uint8_t
		{
			NORMAL,
			MOUSEOVER,
			PRESSED,
			DISABLED,
			NUM_FACES
		};

		Rectangle<int16_t> box;
		Rectangle<int16_t> edit_bounds;
		Textfield edit;
		Texture faces[NUM_FACES][3];
		std::vector<std::string> items;
		std::vector<Text> rows;
		std::vector<Text> lit_rows;
		bool removable = false;
		bool open = false;
		size_t selected = 0;
		// The item last copied into the edit: the edit follows the
		// selection only when it changes (FUN_004c67ad).
		std::optional<size_t> shown;
		size_t hover = 0;
		bool hovered = false;
		uint16_t pressed_time = 0;
		bool mouse_down = false;
	};
}
