#pragma once

#include "../UIElement.h"

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"
#include "../Graphics/Text.h"

#include "../Character/Inventory/Inventory.h"

#include "../Net/Handlers/WeddingGifts.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// Pila's two windows on a couple's gifts (v83 0x9da789 and 0x9dab15,
	// report b084). The Wedding Registry shows a guest the wishlist and gives
	// one of the guest's items; the gift window shows the couple what they
	// were given and takes it out. Both show the player's own items by tab,
	// and say WEDDING_ACTION 8 when they close.
	class UIWeddingRegistry : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::WEDDINGREGISTRY;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		enum class Kind : uint8_t
		{
			GIVE,
			RECEIVE
		};

		UIWeddingRegistry(Kind kind, bool female);

		void draw(float alpha) const override;

		void doubleclick(Point<int16_t> cursorpos) override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> position) override;

		// Tabs and rows.
		bool own_cursor(uint16_t buttonid) const override
		{ return buttonid >= Buttons::TAB0; }

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		Kind get_kind() const;

		// The wishlist the guest gives from.
		void show_wishes(const std::vector<std::string> &wishes);

		// The gifts given, as a result lists them.
		void read_gifts(InPacket &recv);

		// The gift went: v83 says so and closes.
		void sent();

		// A gift was taken, and its type's tab comes up.
		void taken();

		// A request was answered, or refused.
		void answered();

		// The player's bag of a type changed.
		void modify(InventoryType::Id type);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		struct Row
		{
			int32_t itemid;
			int16_t count;
			// An item's inventory slot, or a gift's place in its type's list.
			int16_t slot;
			Texture icon;
			// The name, wrapped onto two lines.
			Text name[2];
		};

		struct List
		{
			std::vector<Row> rows;
			int16_t offset = 0;
			int16_t selection = -1;
		};

		void change_tab(InventoryType::Id tab);

		void refresh_gifts();

		void refresh_items();

		void draw_list(const List &list, Point<int16_t> origin, int16_t rows) const;

		// The row of a list under the cursor, or -1.
		static int16_t row_at(Point<int16_t> cursor, Point<int16_t> origin, int16_t rows);

		void show_item(Point<int16_t> cursor);

		void clear_tooltip();

		bool can_request() const;

		void give();

		void take();

		void close();

		static Row make_row(int32_t itemid, int16_t count, int16_t slot);

		static constexpr int16_t WISH_ROWS = 5;
		static constexpr int16_t ITEM_ROWS = 4;
		static constexpr int16_t MAX_GIFT_ROWS = 5;

		enum Buttons : uint16_t
		{
			EXIT,
			ACTION,
			TAB0,
			TAB4 = TAB0 + 4,
			GIFT0,
			GIFT4 = GIFT0 + MAX_GIFT_ROWS - 1,
			ITEM0,
			ITEM3 = ITEM0 + ITEM_ROWS - 1,
			NUM_BUTTONS
		};

		Kind kind;
		const Inventory &inventory;

		Texture background;
		Texture selected_row;
		ClassicTabs tabs;
		Slider wish_slider;
		Slider gift_slider;
		Slider item_slider;

		std::vector<Text> wishes;
		int16_t wish_offset;

		wedding_gifts::Gifts gifts;
		// The type of the gift asked for last.
		InventoryType::Id taking;

		InventoryType::Id tab;
		// What was given, of every type.
		List given;
		// The player's own items of the tab.
		List carried;

		// A request is out, and the next waits for its answer.
		bool pending;

		Point<int16_t> lastcursorpos;
	};
}
