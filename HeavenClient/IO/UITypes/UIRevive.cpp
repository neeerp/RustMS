#include "UIRevive.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// At (257, 97) on v83's 800 pixel wide screen.
		constexpr int16_t WIDTH = 286, HEIGHT = 146, TOP = 97;
		const Point<int16_t> OK_AT(124, 115);
		// FUN_00898203 answers for the player after ten minutes.
		constexpr uint32_t TIMEOUT = 600000 / Constants::TIMESTEP;
	}

	UIRevive::UIRevive() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT))
	{
		sprites.emplace_back(NxFiles::ui_classic["UIWindow.img"]["Notice"]["0"]);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"], OK_AT);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIRevive::update()
	{
		UIElement::update();

		if (active && ++ticks > TIMEOUT)
			revive();
	}

	void UIRevive::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>((new_width - WIDTH) / 2, TOP);
	}

	UIElement::Type UIRevive::get_type() const
	{
		return TYPE;
	}

	Button::State UIRevive::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			revive();

		return Button::State::NORMAL;
	}

	void UIRevive::revive()
	{
		// Back to the map's return map with a little HP.
		ChangeMapPacket(true, 0, "", false).dispatch();
		deactivate();
	}
}
