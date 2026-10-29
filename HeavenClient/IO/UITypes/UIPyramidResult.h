#pragma once

#include "../UIElement.h"

#include "../../Gameplay/Combat/Massacre.h"
#include "../../Graphics/Animation.h"

#include <array>

namespace ms
{
	// A run's result, in the result room of Nett's Pyramid or the Dusty Platform (CField_MassacreResult, v83
	// 0x5617c5): the kills, cool kills and misses with each one's share of them all, the EXP, and the rank.
	// It shows for 5 seconds and goes; clicks fall through to the field.
	class UIPyramidResult : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PYRAMIDRESULT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// The rank goes 0 for S to 4 for D; any other has none drawn.
		UIPyramidResult(int8_t rank, int32_t exp);

		// Puts up this result in place of any before it.
		static void show(massacre::Result result);

		// The field has loaded: shows a result that came while it loaded.
		static void show_waiting();

		void draw(float alpha) const override;

		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		void draw_number(const std::array<Texture, 10> &font, Point<int16_t> pos, int32_t slots, int16_t advance,
						 int32_t value) const;

		Texture backgrd;
		std::array<Texture, 10> number, number2;
		Animation backgrd2;
		Texture rank;

		std::array<int32_t, 3> counts;
		int32_t exp;
		int16_t shown;
	};
}
