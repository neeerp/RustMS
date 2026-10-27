#pragma once

#include "../UIElement.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Character/Inventory/Raise.h"

#include <nlnx/node.hpp>

#include <map>
#include <string>
#include <vector>

namespace ms
{
	// The raise window (v83 CUIRaise on UIWindow.img/raise, report b233) of an item that grows, as the Timer's Egg:
	// its stage's art in a frame, fed by dropping the items it takes on it. The server sends its record as a quest
	// record message, and the record's quest ending or going closes it.
	class UIRaise : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::RAISE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Opens the window of `item_id` on its double-click (0x892fbd), or brings it to the front. An item whose
		// info names no record or art, or doesn't grow by exp, opens nothing.
		static void open(int32_t item_id);

		// A quest record message for `record` (0xa21332, 0xa215b4): its value while started, else the window
		// goes.
		static void record_changed(int16_t record, int8_t status, const std::string &value);

		UIRaise(int32_t item_id, int16_t record, raise::Growth growth, nl::node art, std::vector<std::string> messages,
				std::vector<int32_t> feed);

		void draw(float alpha) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void set_value(int32_t value);

		// Sizes the window to the art (0x8923e2), and moves the close button with it (0x89212d).
		void resize();

		void close();

		enum Buttons : uint16_t
		{
			CLOSE
		};

		int32_t item_id;
		int16_t record;
		raise::Growth growth;
		nl::node art_src;
		std::vector<std::string> messages;
		std::vector<int32_t> feed;
		int32_t value;
		Texture art;
		// The frame's rows (top, center, bottom), each left, center and right.
		Texture frame[3][3];
		bool dragged;
		Point<int16_t> drag_from;

		// Where each item's window was last, as v83 keeps it (0x893162).
		static std::map<int32_t, Point<int16_t>> positions;
	};
}
