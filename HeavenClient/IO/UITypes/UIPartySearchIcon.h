#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Texture.h"

namespace ms
{
	// The icon at the top right of the screen while a party search runs
	// (v83 0x8799a2, report b034): animated while searching, still while
	// paused. A left click pauses or resumes the search, a right click
	// stops it.
	class UIPartySearchIcon : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PARTYSEARCHICON;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIPartySearchIcon();

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void rightclick(Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// Fade to the searching or the paused icon.
		void show(bool searching);
		// Fade out, then go (0xa11e46).
		void fade_out();

	private:
		Animation searching_icon;
		Texture paused_icon;
		bool searching = false;
		bool going = false;
		bool pressed = false;
		// Each icon's opacity, which moves to its target over 300 ms.
		float searching_opacity = 0.0f;
		float paused_opacity = 0.0f;
	};
}
