#include "UIMobGage.h"

#include "UIMiniMap.h"

#include "../UI.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 copies each canvas with its top left at the given point.
		void draw_at(const Texture &texture, Point<int16_t> corner)
		{
			texture.draw(corner + texture.get_origin());
		}
	}

	UIMobGage::UIMobGage(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor)
			: UIElement(Point<int16_t>(), Point<int16_t>(), false), layout(), hovered(false), tip_y(0)
	{
		set(mobid, hp, maxhp, color, bgcolor);
	}

	void UIMobGage::show(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor)
	{
		if (auto gage = UI::get().get_element<UIMobGage>())
			gage->set(mobid, hp, maxhp, color, bgcolor);
		else if (hp >= 1)
			UI::get().emplace<UIMobGage>(mobid, hp, maxhp, color, bgcolor);
	}

	void UIMobGage::set(int32_t mobid, int32_t hp, int32_t maxhp, uint8_t color, uint8_t bgcolor)
	{
		if (hp < 1)
		{
			hide();

			return;
		}

		// The bar starts at the minimap's right edge, read once per packet (0x533705).
		auto minimap = UI::get().get_element<UIMiniMap>();
		int16_t left = minimap && minimap->is_active() ? minimap->get_width() : 0;

		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MobGage"];
		nl::node gage = src["Gage"];

		backgrnd = src["backgrnd"];
		trough_left = src["backgrnd2"];
		trough = src["backgrnd3"];
		trough_right = src["backgrnd4"];
		icon = src["Mob"][std::to_string(mobid)];
		color_cap = gage[std::to_string(color)]["0"];
		color_column = gage[std::to_string(color)]["1"];
		empty_cap = gage[std::to_string(bgcolor)]["0"];
		empty_column = gage[std::to_string(bgcolor)]["1"];

		// Without an icon there's nothing to hover, where v83 keeps the last mob's (0x534439).
		name = icon.is_valid() ? std::string(nl::nx::string["Mob.img"][std::to_string(mobid)]["name"]) : "";
		tip.set_line(name);

		layout = mob_gage::layout(left, hp, maxhp);
		position = Point<int16_t>(left, 0);
		dimension = Point<int16_t>(800 - left, backgrnd.height());

		makeactive();
	}

	void UIMobGage::hide()
	{
		hovered = false;
		deactivate();
	}

	void UIMobGage::hover(Point<int16_t> cursorpos)
	{
		hovered = active && !name.empty() &&
				  mob_gage::over_icon(position.x(), icon.width(), icon.height(), cursorpos.x(), cursorpos.y());
		tip_y = cursorpos.y();
	}

	void UIMobGage::draw_columns(const Texture &column, int32_t from, int32_t count, int16_t y) const
	{
		// An overfull bar runs off the screen.
		int32_t shown = std::min<int32_t>(count, Constants::Constants::get().get_viewwidth());

		auto x = static_cast<int16_t>(mob_gage::START + from);

		if (shown > 0)
			column.draw(DrawArgument(position + Point<int16_t>(x, y) + column.get_origin(),
									 Point<int16_t>(static_cast<int16_t>(shown), 0)));
	}

	void UIMobGage::draw(float) const
	{
		using namespace mob_gage;

		draw_at(backgrnd, position);
		draw_at(icon, position + Point<int16_t>(ICON_X, ICON_Y));
		draw_at(trough_left, position + Point<int16_t>(TROUGH_X, 0));
		draw_columns(trough, 0, layout.width, 0);
		draw_at(trough_right, position + Point<int16_t>(START + layout.width, 0));

		draw_at(color_cap, position + Point<int16_t>(CAP_X, FILL_Y));
		draw_columns(color_column, 0, layout.fill, FILL_Y);

		if (layout.empty_left)
			draw_at(empty_cap, position + Point<int16_t>(CAP_X, FILL_Y));

		if (layout.empty_from < Constants::Constants::get().get_viewwidth())
			draw_columns(empty_column, layout.empty_from, layout.empty, FILL_Y);

		if (layout.right_x < Constants::Constants::get().get_viewwidth())
			draw_at(layout.full_right ? color_cap : empty_cap,
					position + Point<int16_t>(static_cast<int16_t>(layout.right_x), FILL_Y));

		if (hovered)
			tip.draw_box(Point<int16_t>(position.x() + icon.width() + TIP_GAP, tip_y));
	}

	bool UIMobGage::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIMobGage::get_type() const
	{
		return TYPE;
	}
}
