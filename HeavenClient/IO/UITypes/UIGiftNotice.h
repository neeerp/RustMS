#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A present waiting in the Cash Inventory, as the shop announces it on
	// entering.
	struct CashGift
	{
		int64_t cash_id;
		int32_t item_id;
		std::string sender;
		std::string message;
	};

	// What arrived as a present (v83 7e5ce4, 7e5f63, 7e65de): the item, who
	// sent it and their message, and a note to send back. OK or Enter sends
	// the note, Escape doesn't; either way the next present follows. The
	// items are already in the Cash Inventory.
	class UIGiftNotice : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GIFTNOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// Shows the index-th of the presents.
		UIGiftNotice(std::vector<CashGift> gifts, size_t index);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Set out the index-th present.
		void show(size_t at);

		void send_note();

		// Close without a note and show the next present, if any.
		void skip();

		enum Buttons : uint16_t
		{
			BT_OK
		};

		std::vector<CashGift> gifts;
		size_t index;
		Texture background;
		Textfield note;
		Text name;
		Text sender;
		Text sent_you;
		Text line1;
		Text line2;
	};
}
