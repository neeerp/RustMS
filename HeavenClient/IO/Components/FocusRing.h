#pragma once

#include "Button.h"
#include "Textfield.h"

#include <cstdint>
#include <optional>
#include <vector>

namespace ms
{
	// The controls of a v83 dialog that Tab moves the keys through, in the order they were made (FUN_004edc29). A
	// field takes them as text; a button shows its keyFocused art and is clicked by Space, pressed going down and
	// clicked coming up (FUN_004c0933); a blank stop, such as the megaphone's checkbox, only holds them. The fields and
	// buttons must not move while the ring points at them.
	class FocusRing
	{
	public:
		void add(Textfield &field);

		void add(uint16_t id, Button &button);

		void add_blank();

		// Tab, or Shift+Tab with `back`.
		void cycle(bool back);

		// Gives the keys to button `id`, as a click on it or v83's own code does.
		void focus_button(uint16_t id);

		// Keeps up with a field clicked into.
		void update();

		// Space going down or up. Gives the id of the button it clicks, if any.
		std::optional<uint16_t> space(bool pressed);

		// Whether a stop other than a field holds the keys.
		bool holds_keys() const;

	private:
		struct Stop
		{
			Textfield *field;
			Button *button;
			uint16_t id;
		};

		std::optional<size_t> current() const;

		bool accepts(size_t index) const;

		void focus(size_t index);

		// Takes the keys from whatever stop other than a field has them.
		void let_go();

		std::vector<Stop> stops;
		// The stop other than a field with the keys.
		std::optional<size_t> held;
		bool spacing = false;
	};
}
