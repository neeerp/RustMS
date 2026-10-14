#include "UIGameOption.h"

#include "../KeyAction.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		// Measured from the background: a row for each request, 18 apart, with
		// its box right of the row's ALLOW.
		constexpr int16_t ROW_TOP = 32;
		constexpr int16_t ROW_PITCH = 18;
		constexpr Point<int16_t> BOX = Point<int16_t>(128, 2);
		// The buttons stand at the right of the bar along the bottom.
		constexpr Point<int16_t> BT_OK = Point<int16_t>(51, 259);
		constexpr Point<int16_t> BT_CANCEL = Point<int16_t>(102, 259);

		Point<int16_t> box_position(size_t row)
		{
			return BOX + Point<int16_t>(0, ROW_TOP + ROW_PITCH * static_cast<int16_t>(row));
		}
	}

	UIGameOption::UIGameOption() : UIDragElement<PosGAMEOPTION>()
	{
		nl::node GameOpt = NxFiles::ui_classic["UIWindow.img"]["GameOpt"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		Texture backgrnd = GameOpt["backgrnd"];
		sprites.emplace_back(GameOpt["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Basic["BtOK2"], BT_OK);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], BT_CANCEL);

		box[false] = Basic["CheckBox"]["0"];
		box[true] = Basic["CheckBox"]["1"];

		// In the order of the rows.
		settings = {
			&Setting<AllowWhisper>::get(),
			&Setting<AllowFriends>::get(),
			&Setting<AllowChatInvite>::get(),
			&Setting<AllowTradeRequest>::get(),
			&Setting<AllowPartyInvite>::get(),
			&Setting<AllowPartySearch>::get(),
			&Setting<AllowGameInvite>::get(),
			&Setting<AllowGuildChat>::get(),
			&Setting<AllowGuildInvite>::get(),
			&Setting<AllowAllianceTournament>::get(),
			&Setting<AllowAllianceInvite>::get(),
			&Setting<AllowFamilyInvite>::get()
		};

		for (size_t i = 0; i < ROWS; i++)
		{
			allowed[i] = settings[i]->load();
			buttons[Buttons::BOX + i] = std::make_unique<AreaButton>(box_position(i), box[false].get_dimensions());
		}

		dimension = backgrnd.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);
	}

	void UIGameOption::draw(float inter) const
	{
		UIElement::draw_sprites(inter);

		for (size_t i = 0; i < ROWS; i++)
			box[allowed[i]].draw(position + box_position(i));

		UIElement::draw_buttons(inter);
	}

	void UIGameOption::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
				deactivate();
			else if (keycode == KeyAction::Id::RETURN)
				button_pressed(Buttons::OK);
		}
	}

	UIElement::Type UIGameOption::get_type() const
	{
		return TYPE;
	}

	Button::State UIGameOption::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				save();
				deactivate();
				break;
			case Buttons::CANCEL:
				deactivate();
				break;
			default:
				if (buttonid >= Buttons::BOX && buttonid < Buttons::BOX + ROWS)
					allowed[buttonid - Buttons::BOX] = !allowed[buttonid - Buttons::BOX];

				break;
		}

		return Button::State::NORMAL;
	}

	void UIGameOption::save()
	{
		for (size_t i = 0; i < ROWS; i++)
			settings[i]->save(allowed[i]);
	}
}
