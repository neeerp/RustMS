#include "UIDojoEnergy.h"

#include "../UI.h"

#include "../../Gameplay/Combat/DojoEnergy.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where 0x554005 puts each layer.
		const Point<int16_t> EMPTY(20, 130);
		const Point<int16_t> GAGE(17, 86);
		const Point<int16_t> FULL(9, 80);
	}

	UIDojoEnergy::UIDojoEnergy() : UIElement(Point<int16_t>(), Point<int16_t>())
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["muruengRaid"]["energy"];

		empty = src["empty"]["0"];
		gage = src["empty"]["Gage"]["0"];
		full = src["full"];
	}

	void UIDojoEnergy::show_on(int32_t fieldtype)
	{
		if (pq_skills::allowed(pq_skills::Kind::DOJO_BUFF, fieldtype))
		{
			if (!UI::get().get_element<UIDojoEnergy>())
				UI::get().emplace<UIDojoEnergy>();
		}
		else
		{
			UI::get().remove(TYPE);
		}
	}

	void UIDojoEnergy::draw(float alpha) const
	{
		int32_t energy = dojo_energy::session().energy;

		if (energy >= dojo_energy::FULL)
		{
			full.draw(position + FULL, alpha);

			return;
		}

		empty.draw(position + EMPTY);

		// The dark strip, stretched over the part not yet filled.
		if (int16_t cover = dojo_energy::cover(energy))
			gage.draw(DrawArgument(position + GAGE, Point<int16_t>(0, cover)));
	}

	void UIDojoEnergy::update()
	{
		UIElement::update();

		full.update();
	}

	bool UIDojoEnergy::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIDojoEnergy::get_type() const
	{
		return TYPE;
	}
}
