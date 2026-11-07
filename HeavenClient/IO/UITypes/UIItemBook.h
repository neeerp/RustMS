#pragma once

#include "UINpcTalk.h"

#include "../UIElement.h"

#include "../ItemBook.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <nlnx/node.hpp>

#include <vector>

namespace ms
{
	// The Book window of an Etc 416xxxx item, as a guide or Dran's Note (v83, ctor 0x461a9a, report p332): an
	// open book at (190, 148) showing two pages of the item's `book` at a time, the even one on the left, turned by
	// Prev and Next and by #L links to a page. It is modeless and asks the server nothing.
	class UIItemBook : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMBOOK;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// A double click on the item (0xa10617): opens the window at its first spread, or shows another book's in
		// it. The same book again does nothing.
		static void open(int32_t item_id);

		UIItemBook(int32_t item_id);

		void draw(float alpha) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// A #L<n>#..#l link: the page it goes to, and where its dot's slot starts and its text ends.
		struct Link
		{
			int32_t page;
			Point<int16_t> start;
			Point<int16_t> end;
		};

		// One `book/<page>/<k>` entry, laid out from `at` on its page.
		struct Entry
		{
			Text text;
			std::vector<UINpcTalk::InlineImage> images;
			std::vector<Link> links;
			Point<int16_t> at;
		};

		struct Page
		{
			bool shown = false;
			std::vector<Entry> entries;
			Text number;
		};

		void set_item(int32_t item_id);

		// Shows the spread from `first` (0x462dab).
		void set_page(int32_t first);

		Entry entry(const std::string &text, item_book::Align align, int16_t top) const;

		// The page and link under the cursor, as {page side, entry, link}, or a side of -1.
		struct LinkAt
		{
			int16_t side = -1;
			size_t entry = 0;
			size_t link = 0;
		};

		LinkAt link_at(Point<int16_t> cursor) const;

		bool link_contains(const Link &link, int16_t at, Point<int16_t> cursor) const;

		void close();

		enum Buttons : uint16_t
		{
			PREV,
			NEXT,
			CLOSE
		};

		int32_t item_id;
		nl::node book;
		int32_t last;
		int32_t page;
		Page pages[2];
		Texture background;
		Texture dot;
		Texture dot_hovered;
		int16_t line;
		LinkAt hovered;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
