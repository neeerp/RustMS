#include "UIGuildMakeAgreement.h"
#include "UIChatBar.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 draws its text about 6 pixels higher than ours.
		constexpr int16_t TEXT_RAISE = 6;
		const Point<int16_t> WINDOW_AT = Point<int16_t>(275, 100);
		const Point<int16_t> WINDOW_SIZE = Point<int16_t>(250, 314);
		const Point<int16_t> MESSAGE_AT = Point<int16_t>(31, 51);
		// Both names end at this x on the contract, each written into it
		// twice.
		constexpr int16_t NAMES_RIGHT = 105;
		constexpr int16_t MASTER_Y[] = { 41, 110 };
		constexpr int16_t GUILD_Y[] = { 79, 148 };
		const Point<int16_t> AGREE_AT = Point<int16_t>(28, 219);
		const Point<int16_t> DISAGREE_AT = Point<int16_t>(147, 226);
	}

	UIGuildMakeAgreement::UIGuildMakeAgreement(const std::string& master_name, const std::string& guild_name) : UIElement(WINDOW_AT, WINDOW_SIZE)
	{
		nl::node make = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Guild"]["Make"];

		scroll = Animation(make["backgrnd"]);
		scroll.play(Animation::End::HOLD);
		message = make["message"];

		master = Text(Text::Font::A12B, Text::Alignment::RIGHT, Color::Name::YELLOW, master_name);
		guild = Text(Text::Font::A12B, Text::Alignment::RIGHT, Color::Name::YELLOW, guild_name);

		buttons[Buttons::BT_AGREE] = std::make_unique<MapleButton>(make["BtAgree"], AGREE_AT);
		buttons[Buttons::BT_DISAGREE] = std::make_unique<MapleButton>(make["BtDisagree"], DISAGREE_AT);
		buttons[Buttons::BT_AGREE]->set_active(false);
		buttons[Buttons::BT_DISAGREE]->set_active(false);
	}

	void UIGuildMakeAgreement::draw(float inter) const
	{
		scroll.draw(DrawArgument(position), inter);

		if (timeline.text_shown())
		{
			float opacity = timeline.text_opacity();

			message.draw(DrawArgument(position + MESSAGE_AT, opacity));

			for (int16_t y : MASTER_Y)
				master.draw(DrawArgument(position + MESSAGE_AT + Point<int16_t>(NAMES_RIGHT, y - TEXT_RAISE), opacity));

			for (int16_t y : GUILD_Y)
				guild.draw(DrawArgument(position + MESSAGE_AT + Point<int16_t>(NAMES_RIGHT, y - TEXT_RAISE), opacity));
		}

		UIElement::draw(inter);
	}

	void UIGuildMakeAgreement::update()
	{
		UIElement::update();

		scroll.update();

		if (timeline.update(Constants::TIMESTEP))
		{
			// Said before the refusal goes (0x14d).
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("Since the time has exceeded, denial message has been sent to the party leader.", UIChatBar::LineType::SKILL);

			answer(false);
		}

		if (timeline.buttons_shown())
		{
			buttons[Buttons::BT_AGREE]->set_active(true);
			buttons[Buttons::BT_DISAGREE]->set_active(true);
		}
	}

	void UIGuildMakeAgreement::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// The keys work while the buttons are still hidden.
		if (!pressed)
			return;

		if (escape)
			answer(false);
		else if (keycode == KeyAction::Id::RETURN)
			answer(true);
	}

	UIElement::Type UIGuildMakeAgreement::get_type() const
	{
		return TYPE;
	}

	Button::State UIGuildMakeAgreement::button_pressed(uint16_t buttonid)
	{
		answer(buttonid == Buttons::BT_AGREE);

		return Button::State::NORMAL;
	}

	void UIGuildMakeAgreement::answer(bool agree)
	{
		if (answered)
			return;

		answered = true;
		AgreeToGuildPacket(Stage::get().get_player().get_oid(), agree).dispatch();
		deactivate();
	}
}
