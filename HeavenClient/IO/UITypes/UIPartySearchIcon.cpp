#include "UIPartySearchIcon.h"

#include "../PartySearchActions.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// v83 puts the icon 3 px in from the right and 23 px below its own
		// height (a11279..a11294).
		constexpr int16_t RIGHT = 797;
		constexpr int16_t BELOW = 23;
		constexpr float FADE_STEP = static_cast<float>(Constants::TIMESTEP) / 300.0f;

		float approach(float opacity, bool shown)
		{
			return shown ? std::min(opacity + FADE_STEP, 1.0f) : std::max(opacity - FADE_STEP, 0.0f);
		}
	}

	UIPartySearchIcon::UIPartySearchIcon() : UIElement(Point<int16_t>(), Point<int16_t>(), false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["PartySearch"];

		// icon0 has a sixth frame v83 leaves out.
		std::vector<Frame> frames;

		for (int i = 0; i < 5; i++)
			frames.emplace_back(src["icon0"][std::to_string(i)]);

		searching_icon = Animation(frames);
		paused_icon = src["icon1"];

		dimension = paused_icon.get_dimensions();
		position = Point<int16_t>(RIGHT - dimension.x(), dimension.y() + BELOW);
	}

	void UIPartySearchIcon::draw(float alpha) const
	{
		if (searching_opacity > 0.0f)
			searching_icon.draw(DrawArgument(position, searching_opacity), alpha);

		if (paused_opacity > 0.0f)
			paused_icon.draw(DrawArgument(position, paused_opacity));
	}

	void UIPartySearchIcon::update()
	{
		searching_icon.update();

		searching_opacity = approach(searching_opacity, !going && searching);
		paused_opacity = approach(paused_opacity, !going && !searching);

		if (going && searching_opacity <= 0.0f && paused_opacity <= 0.0f)
			deactivate();
	}

	Cursor::State UIPartySearchIcon::send_cursor(bool clicked, Point<int16_t>)
	{
		if (going)
			return Cursor::State::IDLE;

		// v83 acts on the button's release (0x879ae7).
		if (clicked)
		{
			pressed = true;

			return Cursor::State::CLICKING;
		}

		if (pressed)
		{
			pressed = false;

			if (searching)
				PartySearchActions::pause();
			else
				PartySearchActions::resume();
		}

		return Cursor::State::CANCLICK;
	}

	void UIPartySearchIcon::remove_cursor()
	{
		UIElement::remove_cursor();

		pressed = false;
	}

	void UIPartySearchIcon::rightclick(Point<int16_t>)
	{
		if (!going)
			PartySearchActions::stop();
	}

	UIElement::Type UIPartySearchIcon::get_type() const
	{
		return TYPE;
	}

	void UIPartySearchIcon::show(bool s)
	{
		searching = s;
		going = false;

		makeactive();
	}

	void UIPartySearchIcon::fade_out()
	{
		going = true;
		pressed = false;
	}
}
