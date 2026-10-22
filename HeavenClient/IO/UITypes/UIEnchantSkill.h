#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"

#include <array>

namespace ms
{
	// Legendary Spirit's window (v83 CUIEnchantSkill, 0x7f5abb on UIWindow.img/EnchantSkill): an equip dragged in
	// from the Equip tab, then a scroll from the Use tab, which is sent at once with a White Scroll if the box is
	// ticked. It has no other button than Cancel and Close, and stays open with the equip for another scroll.
	class UIEnchantSkill : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ENCHANTSKILL;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIEnchantSkill();

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// SHOW_SCROLL_EFFECT's answer to the player's own request (FUN_007f651c): -1 if refused, else whether the
		// scroll worked.
		void receive(int8_t result, bool cursed);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The scroll in USE `slot` goes on the equip.
		void scroll(int16_t slot);

		// Once the effect has played (FUN_007f675d).
		void finish();

		void lock(bool locked);

		void close();

		bool holds_white_scroll() const;

		enum Buttons : uint16_t
		{
			CANCEL,
			CLOSE,
			WHITE
		};

		// Its slot in the Equip tab, 0 until one is dropped in.
		int16_t equip_slot;
		bool locked;
		// The White Scroll box is ticked.
		bool white;
		// A White Scroll went with the scroll waiting for its answer.
		bool white_sent;
		bool success;
		bool cursed;
		bool playing;
		std::array<Texture, 3> say;
		// The box: clear, ticked.
		std::array<Texture, 2> box;
		Animation effect;
	};
}
