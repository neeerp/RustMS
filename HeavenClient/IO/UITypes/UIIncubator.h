#pragma once

#include "../UIElement.h"

#include "../../Character/Inventory/InventoryType.h"

namespace ms
{
	// The Incubator's window (v83 0x81a7b9 on UIWindow.img/Incubator, report c035): a Pigmy Egg, put in by
	// double-clicking it or dragged in from the Item window, and OK, which asks, then sends the incubator with it.
	class UIIncubator : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::INCUBATOR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the incubator `item_id` in Cash slot `slot`, with the egg in `egg_tab` and `egg_slot` in, if any.
		UIIncubator(int16_t slot, int32_t item_id, InventoryType::Id egg_tab, int16_t egg_slot);

		void draw(float alpha) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The egg's id while it is still in its place, else 0.
		int32_t egg() const;

		void put(InventoryType::Id tab, int16_t slot);

		void ok();

		void incubate();

		void close();

		void clear_tooltip();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		int16_t slot;
		int32_t item_id;
		// Where the egg put in is, NONE while there is none.
		InventoryType::Id egg_tab;
		int16_t egg_slot;
		int32_t egg_id;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
