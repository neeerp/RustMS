#pragma once

#include "../UIElement.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"
#include "../../Net/Handlers/OwlResults.h"

#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// The Owl of Minerva's results (v83 FUN_008a2482, report c026): the
	// shops selling the item, eight a page, with GO on those on the
	// player's channel that are still open.
	class UIItemSearchResult : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMSEARCHRESULT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		static void open(owl::Results &&results);

		UIItemSearchResult(owl::Results &&results);

		void draw(float alpha) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The server's answer to GO (OWL_GO_RESULT, FUN_008a4e7a).
		void go_result(int8_t code);

		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Fills the page's rows and their GO buttons (FUN_008a4c6c).
		void set_page(int16_t page);
		// Whether GO may take the player to the row's shop.
		bool can_go(const owl::Row &row) const;
		void show_tooltip(Point<int16_t> cursorpos);

		struct Line
		{
			Text owner;
			Text title;
			Text per_bundle;
			Text price;
			Text bundles;
			Text status;
			bool equip;
		};

		enum Buttons : uint16_t
		{
			BT_OK,
			BT_PREV,
			BT_NEXT,
			// One GO for each row of the page.
			BT_GO
		};

		owl::Results results;
		Texture info;
		// The text box above the list, piece by piece.
		std::vector<std::pair<Point<int16_t>, Text>> header;
		std::vector<Line> lines;
		Text page_text;
		int16_t page = 0;
		int16_t pages = 1;
		// The row whose GO waits on its answer.
		int32_t going = -1;
	};
}
