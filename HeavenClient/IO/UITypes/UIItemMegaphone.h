#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Character/Inventory/InventoryType.h"
#include "../../Template/Optional.h"

#include <array>
#include <functional>
#include <string>

namespace ms
{
	// The item megaphone's window (v83 FUN_005eb36d on UIWindow.img/ItemMegaphone): a line, an item dragged in from
	// the inventory to show with it, and whether listeners get an ear to whisper back by. Unlike the other megaphones
	// it leaves the game playable, and sends the line itself.
	class UIItemMegaphone : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ITEMMEGAPHONE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Uses the megaphone `item_id` in Cash slot `slot`.
		UIItemMegaphone(int16_t slot, int32_t item_id);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// Shows the item in `item_slot` of `tab`, or none for NONE (FUN_005ebd3f).
		void set_item(InventoryType::Id tab, int16_t item_slot);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void send();

		// Sends once the player has said whether to go without an item.
		void send_checked();

		void close();

		// Raises a notice, giving the field's keys back after it.
		void notice(const std::string &text);

		void ask(const std::string &question, std::function<void(UIItemMegaphone &window)> on_yes);

		// The open window after a notice, with its field focused again if it was before.
		static Optional<UIItemMegaphone> after_notice(bool refocus);

		// Lets go of the keys for a notice. Returns whether the field had them.
		bool release_field();

		void clear_tooltip();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		int16_t slot;
		int32_t item_id;
		InventoryType::Id item_tab;
		int16_t item_slot;
		int32_t shown_id;
		Text item_name;
		Textfield field;
		std::array<Texture, 2> checkbox;
		bool ear;
		bool held;
	};
}
