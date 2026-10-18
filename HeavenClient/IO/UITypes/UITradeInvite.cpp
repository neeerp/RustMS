#include "UITradeInvite.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Placed and stacked like the quest-complete notice (FUN_00523408),
		// with v83's text positions (FUN_0052065d) moved up to HC's text tops.
		constexpr int16_t LEFT = 464, ABOVE_BOTTOM = 97, CASCADE = 5;
		const Point<int16_t> ICON(6, 9), HEADING(24, 2), FROM(24, 15);
		const Point<int16_t> OK_AT(174, 5), CANCEL_AT(174, 19);
		// How long from appearing until it fades out unanswered.
		constexpr uint32_t LIFETIME = 180000;
	}

	UITradeInvite::UITradeInvite() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0))
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		background = fade["backgrnd"];
		icon = fade["icon2"];
		heading = Text(Text::A12M, Text::LEFT, Color::Name::WHITE, "Trade request");

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UITradeInvite::draw(float alpha) const
	{
		for (size_t i = 0; i < notices.size(); i++)
		{
			Point<int16_t> at = position + offset(i);
			float opacity = notices[i].life.opacity();

			background.draw(DrawArgument(at, opacity));
			icon.draw(DrawArgument(at + ICON, opacity));
			heading.draw(DrawArgument(at + HEADING, opacity));
			notices[i].from.draw(DrawArgument(at + FROM, opacity));
		}

		// Only the top notice can be reached; the others' buttons lie under it.
		if (!notices.empty())
		{
			float opacity = notices.back().life.opacity();

			for (auto& [id, button] : buttons)
				static_cast<MapleButton*>(button.get())->set_opacity(opacity);
		}

		UIElement::draw_buttons(alpha);
	}

	void UITradeInvite::update()
	{
		bool changed = false;

		for (auto it = notices.begin(); it != notices.end();)
		{
			it->life.update(Constants::TIMESTEP);

			// Running out turns the invite down, as X does (FUN_0051fff5).
			if (!it->answered && it->life.closed())
			{
				it->answered = true;
				MiniRoomDeclinePacket(it->serial, MiniRoomPacket::Decline::DECLINED).dispatch();
			}

			if (it->life.gone())
			{
				it = notices.erase(it);
				changed = true;
			}
			else
			{
				++it;
			}
		}

		if (changed)
			rebuild();

		UIElement::update();
	}

	void UITradeInvite::add(const std::string& inviter, int32_t serial)
	{
		Text from(Text::A12M, Text::LEFT, Color::Name::WHITE, "from '" + inviter + "'");

		notices.push_back({ serial, std::move(from), FadeNotice(LIFETIME), false });
		Sound(Sound::Name::INVITE).play();

		rebuild();
	}

	void UITradeInvite::rebuild()
	{
		buttons.clear();

		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		if (notices.empty())
			return;

		Point<int16_t> at = offset(notices.size() - 1);

		buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], at + OK_AT);
		buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + CANCEL_AT);
	}

	Point<int16_t> UITradeInvite::offset(size_t index) const
	{
		return Point<int16_t>(0, -static_cast<int16_t>(index * CASCADE));
	}

	void UITradeInvite::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(LEFT, new_height - ABOVE_BOTTOM);
	}

	bool UITradeInvite::is_in_range(Point<int16_t> cursorpos) const
	{
		if (notices.empty())
			return false;

		Point<int16_t> top = position + offset(notices.size() - 1);

		return Rectangle<int16_t>(top, position + Point<int16_t>(WIDTH, HEIGHT)).contains(cursorpos);
	}

	Button::State UITradeInvite::button_pressed(uint16_t buttonid)
	{
		// One press can arrive twice, and a notice fading out ignores more;
		// act on the first.
		if (notices.empty() || notices.back().life.closed())
			return Button::State::NORMAL;

		Notice& notice = notices.back();
		notice.life.close();
		notice.answered = true;

		if (buttonid == OK)
			MiniRoomVisitPacket(notice.serial).dispatch();
		else
			MiniRoomDeclinePacket(notice.serial, MiniRoomPacket::Decline::DECLINED).dispatch();

		return Button::State::NORMAL;
	}

	UIElement::Type UITradeInvite::get_type() const
	{
		return TYPE;
	}
}
