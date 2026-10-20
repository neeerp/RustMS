#include "UIEnergyBar.h"

#include "EnergyBarLayout.h"

#include "../UI.h"

#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// StringPool 0x75e, its line break as Text marks one.
		const char *TIP = "Displays the energy level that's been charged.\\nUse the mouse to change position.";

		// Tooltips go 22 below the cursor.
		const Point<int16_t> TIP_OFFSET = Point<int16_t>(0, 22);

		// v83 copies each canvas with its top left at the given point.
		void draw_at(const Texture &texture, Point<int16_t> corner, float opacity = 1.0f)
		{
			texture.draw(DrawArgument(corner + texture.get_origin(), opacity));
		}
	}

	UIEnergyBar::UIEnergyBar(int32_t e)
			: UIDragElement<PosENERGYBAR>(Point<int16_t>(energy_bar::WIDTH, energy_bar::HEIGHT)), energy(e),
			  hovered(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["EnergyBar"];

		left = src["w"];
		column = src["c"];
		right = src["e"];
		gage_cap = src["Gage"]["1"]["0"];
		gage_column = src["Gage"]["1"]["1"];
		glow = src["effect"];
		tip.set_text(TIP);

		dimension = Point<int16_t>(energy_bar::WIDTH, energy_bar::HEIGHT);
	}

	void UIEnergyBar::show(int32_t e)
	{
		if (e <= 0)
			UI::get().remove(TYPE);
		else if (auto bar = UI::get().get_element<UIEnergyBar>())
			bar->energy = e;
		else
			UI::get().emplace<UIEnergyBar>(e);
	}

	void UIEnergyBar::draw(float alpha) const
	{
		using namespace energy_bar;

		if (energy >= FULL)
		{
			glow.draw(DrawArgument(position + glow.get_origin()), alpha);
		}
		else
		{
			draw_at(left, position, FRAME_OPACITY);

			Point<int16_t> middle = position + Point<int16_t>(END, 0);
			column.draw(DrawArgument(middle + column.get_origin(), middle, Point<int16_t>(WIDTH - 2 * END, 0), 1.0f,
									 1.0f, FRAME_OPACITY, 0.0f));

			draw_at(right, position + Point<int16_t>(WIDTH - END, 0), FRAME_OPACITY);

			int32_t columns = fill(energy);

			for (int16_t x = START; x - START < columns; x++)
			{
				if (cap(x))
					draw_at(gage_cap, position + Point<int16_t>(x, CAP_Y));
				else
					draw_at(gage_column, position + Point<int16_t>(x, COLUMN_Y));
			}
		}

		if (hovered)
			tip.draw(cursor + TIP_OFFSET);
	}

	void UIEnergyBar::update()
	{
		if (energy >= energy_bar::FULL)
			glow.update();
	}

	Cursor::State UIEnergyBar::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		hovered = true;
		cursor = cursorpos;

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIEnergyBar::remove_cursor()
	{
		hovered = false;

		UIDragElement::remove_cursor();
	}

	UIElement::Type UIEnergyBar::get_type() const
	{
		return TYPE;
	}
}
