#pragma once

#include "MobGageLayout.h"

#include "../UIElement.h"

#include "../Components/PlainTooltip.h"

#include <cstdint>
#include <string>

namespace ms
{
	// The boss HP bar along the top of the screen, right of the minimap (FIELD_EFFECT 5, v83 0x5336ca). Each
	// packet rebuilds it. Clicks fall through to the field, and the mob's name shows over its icon (0x529a09).
	class UIMobGage : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MOBGAGE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMobGage(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor);

		// Shows the bar for the mob template `mobid`, or takes it down for an hp under 1.
		static void show(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor);

		void draw(float alpha) const override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		void set(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor);

		void hide();

		// The cursor over the field, which shows the mob's name over its icon.
		void hover(Point<int16_t> cursorpos);

	private:
		// Draws `count` columns of `column` from `from` in the trough.
		void draw_columns(const Texture &column, int32_t from, int32_t count, int16_t y) const;

		mob_gage::Layout layout;
		Texture backgrnd, trough_left, trough, trough_right, icon;
		Texture color_cap, color_column, empty_cap, empty_column;
		std::string name;
		PlainTooltip tip;
		bool hovered;
		int16_t tip_y;
	};
}
