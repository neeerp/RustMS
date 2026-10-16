#pragma once

#include "../UIElement.h"

#include <cstdint>

namespace ms
{
	// v83's "PRESS OK TO BE REVIVED." window (UIWindow.img/Notice/0, FUN_00897b5f). It does not
	// move and ignores the keyboard. OK, or ten minutes without an answer, sends the player to the
	// map's return map.
	class UIRevive : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::REVIVE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIRevive();

		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void revive();

		enum Buttons : uint16_t { OK };

		uint32_t ticks = 0;
	};
}
