#pragma once

#include "../UIElement.h"

#include "../../Character/Inventory/CashUpgrades.h"
#include "../../Character/Inventory/InventoryType.h"
#include "../../Graphics/Animation.h"

namespace ms
{
	// Vicious' Hammer's window (v83 FUN_0082a754 on UIWindow.img/ViciousHammer): an equip dragged in from the Equip tab,
	// and Start, which sends the hammer and fills the gauge. When both the gauge is full and the server has answered, a
	// second later after a success, it tells the server it has shown the result (USE_HAMMER), and the server's last
	// word closes it with a notice. It is dragged by its top.
	class UIViciousHammer : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::VICIOUSHAMMER;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the hammer in Cash slot `slot`.
		UIViciousHammer(int16_t slot);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// What the server said of the hammer (354 with any other code than 0x3d or 0x3e): `flag` 0 if it went on, and
		// how many hammers the equip has had since.
		void answer(int32_t code, int32_t flag, int32_t applied);

		// The server's last word: 0x3d with its flag, or 0x3e with the reason.
		void finish(int32_t flag);

		void refuse(int32_t reason);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void start();

		// Why the hammer won't go on the item in `tab` slot `s`, if it won't.
		static vicious_hammer::Refusal check(InventoryType::Id tab, int16_t s);

		// Sends USE_HAMMER once the gauge is full and the answer is in (FUN_0082b7f6).
		void arm();

		void close();

		void clear_tooltip();

		enum Buttons : uint16_t
		{
			START,
			CANCEL
		};

		enum class State
		{
			IDLE,
			FILLING,
			FULL,
			SHOWN
		};

		int16_t slot;
		// The equip's Equip-tab slot, 0 before one is dragged in.
		int16_t equip_slot;
		State state;
		// Since Start, in ms, and when to send USE_HAMMER, or -1 until armed.
		int32_t elapsed;
		int32_t send_at;
		bool answered;
		int32_t code;
		int32_t flag;
		int32_t applied;
		Texture gauge;
		Animation pressed;
		Animation success;
		// Dragged by the top, from where it was taken.
		bool dragged;
		Point<int16_t> drag_from;
	};
}
