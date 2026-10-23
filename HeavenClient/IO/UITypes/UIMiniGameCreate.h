#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>

namespace ms
{
	// The dialog that opens an Omok room (v83 0xa1d86a, 0x7dc8ef) or a match
	// cards room (0xa1da57, 0x7dd5e4): the room's title, the cards' count,
	// and a password if it is locked.
	class UIMiniGameCreate : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MINIGAMECREATE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// For the game set `itemid`: an Omok set, whose stones the room
		// plays with, or the match cards.
		UIMiniGameCreate(int32_t itemid);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// A game set was double-clicked in the inventory (0xa1d4d8): the
		// dialog opens unless something else is under way or the map
		// forbids games.
		static void open(int32_t itemid);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void toggle_lock();

		void confirm();

		void close();

		bool can_confirm() const;

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			CARD_COUNT
		};

		int32_t itemid;
		bool omok;
		bool locked;

		Texture backgrnd;
		std::array<Texture, 2> lock;
		std::array<Texture, 2> checkbox;
		std::array<Texture, 2> password_box;
		Text password_label;

		Textfield title;
		Textfield password;
	};
}
