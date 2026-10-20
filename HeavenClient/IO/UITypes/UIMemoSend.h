#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Texture.h"

#include <string>

namespace ms
{
	// v83's SEND NOTE dialog (UIWindow.img/Memo/backgrnd3, FUN_007dc20a,
	// report 204m): a receiver and a message, sent with the note item in
	// Cash slot `slot`. OK checks them and stays open on a problem.
	class UIMemoSend : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEMOSEND;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIMemoSend(int16_t slot, int32_t item_id, const std::string& to);

		// Write to `to` with the first note item in the Cash Inventory
		// (FUN_0090aa2c), as the buddy list does.
		static void write_to(const std::string& to);

		void draw(float inter) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CANCEL, CLOSE };

		void send();
		void close();

		int16_t slot;
		int32_t item_id;
		Texture background;
		Textfield receiver, message;
	};
}
