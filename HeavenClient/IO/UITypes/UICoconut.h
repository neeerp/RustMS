#pragma once

#include "../UIElement.h"

#include "../../Graphics/Texture.h"

#include <array>
#include <map>
#include <string>

namespace ms
{
	// Coconut Season's scoreboard (CField_Coconut, v83ref b167): Team
	// Maple's score on the left, Team Story's on the right and the time left
	// under them, at the top right of the screen. Clicks fall through to the
	// field.
	class UICoconut : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::COCONUT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UICoconut();

		// Puts the scoreboard up on a field of this type, or takes it down.
		static void show_on(int32_t fieldtype);

		void draw(float alpha) const override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		// Draws `text` a glyph after another from `pos`, their top left corners in line and `gap` apart
		// (FUN_0098812c).
		void draw_text(const std::map<char, Texture>& font, const std::string& text, Point<int16_t> pos,
					   int16_t gap) const;

		Texture backgrnd;
		std::map<char, Texture> score_font;
		std::map<char, Texture> time_font;
	};
}
