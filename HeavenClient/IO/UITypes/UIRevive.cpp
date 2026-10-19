#include "UIRevive.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/MapleMap/FieldLimit.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// At (257, 97) on v83's 800 pixel wide screen.
		constexpr int16_t WIDTH = 286, HEIGHT = 146, TOP = 97;
		const Point<int16_t> OK_AT(124, 115), YES_AT(92, 115), NO_AT(148, 115);
		// FUN_00898203 answers for the player after ten minutes.
		constexpr uint32_t TIMEOUT = 600000 / Constants::TIMESTEP;
		constexpr int32_t WHEEL_OF_DESTINY = 5510000;
	}

	UIRevive::UIRevive() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, HEIGHT))
	{
		Stage& stage = Stage::get();
		Player& player = stage.get_player();
		nl::node notice = NxFiles::ui_classic["UIWindow.img"]["Notice"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		bool wheel = field_limit::may_use_wheel(stage.get_mapinfo().get_fieldtype(), stage.get_mapid())
			&& player.get_inventory().get_total_item_count(WHEEL_OF_DESTINY) > 0;

		if (wheel)
		{
			UseDeathItemPacket(WHEEL_OF_DESTINY, player.get_position()).dispatch();

			sprites.emplace_back(notice["2"]);
			buttons[Buttons::YES] = std::make_unique<MapleButton>(basic["BtYes2"], YES_AT);
			buttons[Buttons::NO] = std::make_unique<MapleButton>(basic["BtNo2"], NO_AT);
		}
		else
		{
			sprites.emplace_back(notice["0"]);
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		}

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIRevive::update()
	{
		UIElement::update();

		if (active && ++ticks > TIMEOUT)
			revive(false);
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
		revive(buttonid == Buttons::YES);

		return Button::State::NORMAL;
	}

	void UIRevive::revive(bool wheel)
	{
		// Back to the map's return map with a little HP, or here with a wheel.
		ChangeMapPacket(Stage::get().get_field_key(), 0, "", wheel).dispatch();
		deactivate();
	}
}
