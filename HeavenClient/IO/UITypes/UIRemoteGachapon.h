#pragma once

#include "../UIElement.h"

namespace ms
{
	// The Remote Gachapon Ticket's window (v83 0x81104e on UIWindow.img/RemoteGachapon, report b196): a button for
	// each town's machine, which sends the ticket with the town and closes the window. Nothing else is asked.
	class UIRemoteGachapon : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::REMOTEGACHAPON;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// For the ticket `item_id`.
		UIRemoteGachapon(int32_t item_id);

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// After the towns' buttons, 0 to 9.
		static constexpr uint16_t CLOSE = 10;

		int32_t item_id;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
