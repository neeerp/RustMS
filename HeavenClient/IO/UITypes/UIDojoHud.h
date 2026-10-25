#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"

#include <array>

namespace ms
{
	// The Mu Lung Dojo's HUD, up on the fields where its secret skills may be used (v83 0x554005, updated by
	// 0x554792): the player's and the boss's HP gauges and the clock across the top, and the energy bar at the
	// left. The energy strip fills from the bottom; once it is full it flashes with the keys that cast each
	// skill. Clicks fall through to the field.
	class UIDojoHud : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::DOJOHUD;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIDojoHud();

		// Puts the HUD up on a field of this type, or takes it down.
		static void show_on(int32_t fieldtype);

		// The time the HUD's clock counts by, in milliseconds.
		static int64_t now();

		void draw(float alpha) const override;

		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		Texture player_frame, clock_frame, monster_frame, hp_gage;
		std::array<Texture, 10> digits;
		Texture bar;
		Texture empty, gage;
		Animation full;

		int32_t player_width;
		// -1 with no boss on the field.
		int32_t monster_width;
	};
}
