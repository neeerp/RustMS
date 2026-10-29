#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include "../../Character/Ariant.h"
#include "../../Template/Interpolated.h"

#include <array>
#include <vector>

namespace ms
{
	// The Battle Arena's board (v83ref ariant-s4-hud): each player's icon,
	// name and Spirit Jewels at the top left of the screen, highest first,
	// with no frame. At the results the board goes, a scroll unrolls over
	// the field, and the places, names and scores fade in on it
	// (ariant-s4-result). Clicks fall through to the field.
	class UIAriant : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ARIANT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIAriant();

		// Puts the board up on a field of this type, or takes it down.
		static void show_on(int32_t fieldtype);

		// ARIANT_RESULT: the board goes and the scroll unrolls, with the
		// ScrollOpen sound.
		void show_result();

		void draw(float alpha) const override;
		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		std::array<Texture, 6> icons;
		// The rows as last drawn, and their texts.
		std::vector<ariant::Row> shown;
		std::vector<Text> names;
		std::vector<Text> scores;
		Animation result;
		bool showing_result;
		// The scroll's text, written once it has unrolled.
		std::vector<Text> result_places;
		std::vector<Text> result_names;
		std::vector<Text> result_scores;
		bool result_written;
		Linear<float> result_opacity;
	};
}
