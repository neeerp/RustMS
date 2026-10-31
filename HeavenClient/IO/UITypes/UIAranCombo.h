#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// The Aran's combo at the top right (v83 FUN_00960342, report b361-aran-combo-hud.md): the count in
	// digits coloured by its tier, which bounce as it comes, the word "Combo" under them, both fading over
	// the 3 seconds the count lasts, and at 30, 100 and 200 the ComboCommand's lines for the skills it frees.
	// Clicks fall through to the field.
	class UIAranCombo : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ARANCOMBO;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIAranCombo();

		// Shows `count`, with its ComboCommand's first and second lines if so, or takes the HUD down at 0.
		static void show(int32_t count, bool first, bool second);

		// The Lie Detector froze the combo: the count shown again, still and opaque, and so it stays until it
		// ends, thawed or not (FUN_00960fa4 with 1, report b361).
		static void hold();

		void draw(float alpha) const override;
		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		void set(int32_t count, bool first, bool second);

		int32_t count;
		std::vector<Texture> digits;
		Animation word;
		// The ComboCommand's lines, which stay until the count ends.
		Animation first_line, second_line;
		bool first_shown, second_shown;
		// How long ago the count came.
		int64_t elapsed;
		// Drawn without the fade or the bounce.
		bool held;
	};
}
