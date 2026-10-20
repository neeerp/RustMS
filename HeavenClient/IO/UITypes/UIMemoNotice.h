#pragma once

#include "../UIElement.h"
#include "../Components/FadeNotice.h"

#include "../../Graphics/Texture.h"

namespace ms
{
	// v83's "You have a new note" (FadeYesNo type 4, report 204m): it fades
	// in above the status bar when notes come and stays until answered. O
	// opens the inbox and X puts it away; the notes wait either way. Unlike
	// the other fading notices it keeps its own place.
	class UIMemoNotice : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEMONOTICE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMemoNotice();

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;

		UIElement::Type get_type() const override;

		// Whether one is there and not on its way out.
		static bool showing();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CANCEL };

		Texture background;
		FadeNotice life;
	};
}
