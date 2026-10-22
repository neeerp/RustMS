#pragma once

#include "../UIElement.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Character/Inventory/ItemGuard.h"

#include <optional>

namespace ms
{
	// An Item Guard's window (v83 CUIItemProtector on UIWindow.img/ItemProtector, report c009): an item dragged in
	// from the Item window, and OK, which asks before sending the guard. Nothing goes until Yes.
	class UIItemProtector : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMPROTECTOR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the guard `item_id` in Cash slot `slot`, which seals for `days`, or for good with 0.
		UIItemProtector(int16_t slot, int32_t item_id, int32_t days);

		void draw(float alpha) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The item in the target's place now, if any.
		static std::optional<item_guard::Target> target_at(InventoryType::Id tab, int16_t slot);

		void ok();

		void close();

		void clear_tooltip();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		int16_t slot;
		int32_t item_id;
		int32_t days;
		// Where the item dropped in is, NONE while there is none.
		InventoryType::Id target_tab;
		int16_t target_slot;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
