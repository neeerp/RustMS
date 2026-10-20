#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// v83's exception list (FUN_008fc61b): what a pet wearing an Item Ignore
	// Pendant leaves on the ground. The player's own character info opens it
	// beside its pet box, drawn under it, and it edits the list of the pet
	// the box shows. It sends the list each time it changes and when it
	// closes.
	class UIPetExceptions : public UIElement
	{
	public:
		UIPetExceptions(uint8_t pet, Point<int16_t> position);

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_scroll(double yoffset) override;

		UIElement::Type get_type() const override;

		// The character info keeps the cursor here while a button is held.
		using UIElement::is_holding_button;

		void set_position(Point<int16_t> position);
		uint8_t get_pet() const;

		// Add `itemid` in its place by name (FUN_008fcede), or remove it
		// (FUN_008fd126). Either sends the list when it changes.
		void add(int32_t itemid);
		void remove(int32_t itemid);
		// Send the list as it closes (FUN_008fca29), and close the item
		// search with it.
		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void refresh();
		// The row under the cursor (FUN_008fd1f4).
		int16_t row_at(Point<int16_t> cursorpos) const;

		enum Buttons : uint16_t
		{
			BT_HIDE,
			BT_MESO,
			BT_REGISTER,
			BT_DELETE
		};

		uint8_t pet;
		Texture background;
		Slider slider;
		// What the rows show, refreshed when the pet's list changes.
		std::vector<int32_t> shown;
		std::vector<Text> names;
		int16_t offset = 0;
		int16_t hovered = -1;
		int16_t selected = -1;
		bool mouse_down = false;
	};
}
