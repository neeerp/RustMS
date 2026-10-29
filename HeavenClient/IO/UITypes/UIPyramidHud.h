#pragma once

#include "../UIElement.h"

#include "../../Gameplay/Combat/Massacre.h"
#include "../../Graphics/Animation.h"

#include <array>

namespace ms
{
	// The HUD of a stage of Nett's Pyramid or the Dusty Platform (CField_Massacre, v83 0x55f33f, updated by
	// 0x5602c0): the panel counting kills, cool kills and misses at the top left, with the uses of Rage of
	// Pharaoh under it, the gauge that runs down across the top, and the stage's clock above it. Clicks fall
	// through to the field.
	class UIPyramidHud : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PYRAMIDHUD;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIPyramidHud();

		// Puts the HUD up on a field of this type, or takes it down.
		static void show_on(int32_t fieldtype);

		void draw(float alpha) const override;

		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		void draw_number(const std::array<Texture, 10> &font, Point<int16_t> pos, int32_t slots, int16_t advance,
						 int32_t value) const;

		// The map's `mobMassacre`.
		bool skill_disabled;
		int32_t total;
		int32_t decrease;

		Texture count_backgrd;
		std::array<Texture, 10> number, number2;
		Animation key_open, key_ing, key_close;
		massacre::KeyPanel key;
		int32_t last_skill;

		Texture backgrd, text, pixel;
		Animation backgrd_danger, text_danger, danger, icon;
		int16_t width;
		bool in_danger;

		Texture clock_backgrd;
		std::array<Texture, 10> clock_font;
		bool cleared;
	};
}
