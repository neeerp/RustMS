#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// v83's item search for a pet's exception list (FUN_008fc031), opened
	// from the exception window over its corner. A search lists the items
	// whose names have the text below the dialog (FUN_008fb64b), where the
	// selected one can be added to or removed from the list.
	class UIPetItemSearch : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PETITEMSEARCH;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIPetItemSearch(Point<int16_t> position);

		void draw(float alpha) const override;
		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_scroll(double yoffset) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// List what the text names (FUN_008fc432).
		void search(const std::string &text);
		// The result under the cursor (FUN_008fbfa3).
		int16_t row_at(Point<int16_t> cursorpos) const;
		Rectangle<int16_t> results_bounds() const;

		enum Buttons : uint16_t
		{
			BT_SEARCH,
			BT_CLOSE,
			BT_SAVE,
			BT_DELETE
		};

		Texture background;
		Texture results_background;
		Textfield edit;
		// Whether a search has opened the results.
		bool searched = false;
		std::vector<int32_t> results;
		std::vector<Text> names;
		Slider slider;
		int16_t offset = 0;
		int16_t hovered = -1;
		int16_t selected = -1;
		bool mouse_down = false;
		bool dragged = false;
		Point<int16_t> cursoroffset;
		Point<int16_t> cursor;
	};
}
