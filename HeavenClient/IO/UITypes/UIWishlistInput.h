#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"
#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The wishlist the wedding staff ask each of the couple for, as
	// MARRIAGE_REQUEST 9 opens it (v83 0x9d7822, report b084): type an item,
	// Enter it onto the list of ten, Delete the selected one, and OK hands
	// the list in. There is no other way out.
	class UIWishlistInput : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::WISHLISTINPUT;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIWishlistInput(bool female);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		static constexpr size_t MAX_WISHES = 10;

		void enter();

		void remove();

		void submit();

		// Give the keys back to the field once a notice is answered.
		void refocus();

		enum Buttons : uint16_t
		{
			BT_ENTER,
			BT_OK,
			BT_DELETE
		};

		Texture background;
		Texture selected_row;
		Textfield textfield;
		std::vector<std::string> wishes;
		std::vector<Text> labels;
		int16_t selection;
	};
}
