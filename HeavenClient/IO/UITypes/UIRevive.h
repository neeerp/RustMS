#pragma once

#include "../UIElement.h"

#include <cstdint>

namespace ms
{
	// v83's death window (FUN_00897c94). It does not move and ignores the keyboard. With a Wheel of
	// Destiny where one works, it asks whether to revive here (UIWindow.img/Notice/2); YES revives on
	// this map. Otherwise it says "PRESS OK TO BE REVIVED." (Notice/0). OK, NO, or ten minutes
	// without an answer, sends the player to the map's return map.
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
		void revive(bool wheel);

		enum Buttons : uint16_t { OK, YES, NO };

		uint32_t ticks = 0;
	};
}
