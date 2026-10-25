#pragma once

#include "../UIElement.h"

#include "../../Graphics/Animation.h"

namespace ms
{
	// The Mu Lung Dojo's energy bar at the left of the screen, up on the fields where its secret skills may
	// be used (v83 0x554005, updated by 0x554792). The strip fills from the bottom with the energy; once it is
	// full it flashes with the keys that cast each skill. Clicks fall through to the field.
	class UIDojoEnergy : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::DOJOENERGY;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIDojoEnergy();

		// Puts the bar up on a field of this type, or takes it down.
		static void show_on(int32_t fieldtype);

		void draw(float alpha) const override;

		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	private:
		Texture empty, gage;
		Animation full;
	};
}
