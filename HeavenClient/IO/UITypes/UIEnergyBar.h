#pragma once

#include "../UIDragElement.h"

#include "../Components/TextTooltip.h"

#include "../../Graphics/Animation.h"

#include <cstdint>

namespace ms
{
	// The Energy Charge gauge (v83 FUN_007faa49): GIVE_BUFF's energy, from 1 to full, and nothing once it is 0.
	// A full gauge plays its glow instead. It can be dragged anywhere, and says so over it.
	class UIEnergyBar : public UIDragElement<PosENERGYBAR>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::ENERGYBAR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		explicit UIEnergyBar(int32_t energy);

		// Shows the gauge at `energy`, or takes it down at 0.
		static void show(int32_t energy);

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		UIElement::Type get_type() const override;

	private:
		int32_t energy;
		Texture left, column, right, gage_cap, gage_column;
		Animation glow;
		TextTooltip tip;
		bool hovered;
		Point<int16_t> cursor;
	};
}
