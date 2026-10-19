#pragma once

#include "SlideMenu.h"

#include "../UIElement.h"

#include "../../Graphics/Text.h"

#include <nlnx/node.hpp>

#include <cstdint>

namespace ms
{
	// An NPC's slide menu (NPC dialogue type 14; v83 FUN_0076b9fc and FUN_0076be7a on
	// UIWindow.img/SlideMenu/<menu type>), such as the Dimensional Mirror's. It shows three of the choices'
	// icons at a time, the middle one framed and labelled; the arrows, Left and Right slide them along. It is
	// modal: Move, Enter or the middle icon answers with the middle choice; Cancel or Escape answers with
	// none.
	class UISlideMenu : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::SLIDEMENU;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UISlideMenu(int32_t menutype, slide_menu::Menu menu);

		void draw(float alpha) const override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Put the choice `index` in the middle.
		void show(size_t index);

		// Answer with the middle choice, or with none.
		void answer(bool ok);

		enum Buttons : uint16_t
		{
			LEFT,
			RIGHT,
			MOVE,
			CANCEL,
			BEFORE,
			MIDDLE,
			AFTER
		};

		nl::node src;
		slide_menu::Menu menu;
		size_t shown;
		Texture cover;
		Texture choice;
		Text label;
	};
}
