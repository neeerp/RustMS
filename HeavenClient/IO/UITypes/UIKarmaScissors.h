#pragma once

#include "../UIElement.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Character/Inventory/KarmaScissors.h"

#include <optional>

namespace ms
{
	// The Scissors of Karma's window (v83 0x830ad6 on UIWindow.img/KarmaScissors, report c010): an item dragged in
	// from the Item window, and OK, which sends the scissors with it. There is no question.
	class UIKarmaScissors : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::KARMASCISSORS;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the scissors `item_id` in Cash slot `slot`.
		UIKarmaScissors(int16_t slot, int32_t item_id);

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
		static std::optional<karma_scissors::Target> target_at(InventoryType::Id tab, int16_t slot);

		void ok();

		void close();

		void clear_tooltip();

		enum Buttons : uint16_t
		{
			OK,
			CLOSE
		};

		int16_t slot;
		int32_t item_id;
		// Where the item dropped in is, NONE while there is none.
		InventoryType::Id target_tab;
		int16_t target_slot;
		int32_t target_id;
		bool dragged;
		Point<int16_t> drag_from;
	};
}
