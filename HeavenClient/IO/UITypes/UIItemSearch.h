#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The Owl of Minerva's search window (v83 FUN_008a0c7b, report c026):
	// type a name, or pick one of the items searched for most or a category,
	// and a panel slides out to the right with the items to choose from.
	// Choosing one asks first, then closes the window and sends the search.
	class UIItemSearch : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMSEARCH;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Opens the window for the Owl in USE `slot`, unless it is open.
		static void open(int16_t slot, int32_t owl_id);

		UIItemSearch(int16_t slot, int32_t owl_id);

		void draw(float alpha) const override;
		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The items searched for most, from the server (OWL_RESULT mode 7).
		void set_popular(std::vector<int32_t> items);

		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// What slides out to the right (FUN_008a166d's modes).
		enum class Panel
		{
			NONE,
			POPULAR,
			CATEGORIES,
			RESULTS
		};

		// One line of the category tree: a heading that opens or closes
		// its part, or a category to list.
		struct Category
		{
			std::string label;
			bool heading;
			// The part a heading opens, or -1 for a category.
			int8_t opens;
			// The item ids a category takes, either range.
			int32_t low, high, low2, high2;
		};

		// Opens a panel, or slides the open one away (Panel::NONE).
		void show(Panel panel);
		void open_panel(Panel panel);
		// Lists the items whose names have the text (FUN_008a229a).
		void search_name(const std::string &text);
		// Lists the items of a category (FUN_008a0c0e).
		void search_category(const Category &category);
		void set_page(int16_t page);
		// Asks before using the Owl on the item (SP 0xe2d), then sends
		// the search and closes (FUN_008a2407).
		void ask(int32_t item_id);
		std::vector<Category> categories() const;
		// The row of the open panel under the cursor, or -1.
		int16_t row_at(Point<int16_t> cursorpos) const;
		int16_t rows_shown() const;
		int32_t item_at(int16_t row) const;
		Rectangle<int16_t> panel_bounds() const;
		// How far out the panel is, against the window.
		Point<int16_t> panel_offset() const;
		Rectangle<int16_t> order_bounds() const;

		enum Buttons : uint16_t
		{
			BT_POPULAR,
			BT_CATEGORIES,
			BT_SEARCH,
			BT_CLOSE,
			BT_HIDE,
			BT_PREV,
			BT_NEXT,
			BT_CHOOSE
		};

		int16_t slot;
		int32_t owl_id;
		Texture background;
		Texture panel_background;
		Texture order_icon;
		Textfield edit;
		Panel panel = Panel::NONE;
		// How far the panel has slid out, and whether it is sliding back.
		int16_t slide = 0;
		bool hiding = false;
		std::vector<int32_t> popular;
		std::vector<int32_t> found;
		// The part of the category tree that is open (panel C's state).
		int8_t part = 0;
		int16_t page = 0;
		int16_t hovered = -1;
		int16_t selected = -1;
		bool descending = false;
		std::vector<Category> tree;
		std::vector<Text> names;
		Text order;
		bool mouse_down = false;
		bool dragged = false;
		Point<int16_t> cursoroffset;
	};
}
