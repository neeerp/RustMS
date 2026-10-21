#pragma once

#include "../QuickSlots.h"
#include "../UIElement.h"

#include "../../Graphics/Texture.h"

#include <cstddef>
#include <cstdint>
#include <map>

namespace ms
{
	// The quick slot key editor behind Key Config's "CHANGE QUICK SLOTS"
	// (FUN_00836c9b, dialog FUN_0072ca6b). Pick a cell and press a key; OK
	// hands the layout to Key Config, which sends it with its own OK.
	class UIQuickSlotConfig : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::QUICKSLOTCONFIG;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIQuickSlotConfig();

		void draw(float inter) const override;
		void update_screen(int16_t new_width, int16_t new_height) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		// Every key while the editor is up, by its GLFW code (OnKey 72ce91).
		void send_raw_key(int32_t keycode, bool pressed);

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Where the keyboard is (+0x9c): nowhere, on the cells, or on OK
		// and Cancel.
		enum class Focus
		{
			NONE,
			CELLS,
			BUTTONS
		};

		void select(size_t cell);
		void focus_button(bool cancel);
		void unfocus();
		void clear_highlights();
		void press_key(int32_t keycode);
		void ok();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL,
			CELL
		};

		QuickSlots::Keys working;
		Focus focus = Focus::NONE;
		size_t selected = 0;
		bool mouse_down = false;

		std::map<int32_t, Texture> labels;
	};
}
