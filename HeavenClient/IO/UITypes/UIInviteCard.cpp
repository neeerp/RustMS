#include "UIInviteCard.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Net/Handlers/Parcels.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <limits>

namespace ms
{
	namespace
	{
		// Placed like the quest-complete notice (FUN_00523408).
		constexpr int16_t LEFT = 464, ABOVE_BOTTOM = 97;
		const Point<int16_t> OK_AT(174, 5), CANCEL_AT(174, 19);
		// The arrivals' only button, at v83's x 136 like the others are at 177.
		const Point<int16_t> ARRIVAL_CANCEL_AT(133, 19);
		// The icon's left edge and the band it is centred in (FUN_0052065d).
		constexpr int16_t ICON_X = 6, ICON_BAND = 37;
		// How far the "from" line sits below the heading.
		constexpr int16_t LINE = 13;
		// The parcels' buttons, at v83's x 136 like the quest notice's.
		const Point<int16_t> PARCEL_OK_AT(133, 5), PARCEL_CANCEL_AT(133, 19);
		// How long from appearing until it fades out unanswered.
		constexpr uint32_t LIFETIME = 180000, ARRIVAL_LIFETIME = 6000, PARCEL_LIFETIME = 6000;

		struct Look
		{
			const char* icon;
			const char* heading;
			// v83's text position (FUN_0052065d), moved up to HC's text tops.
			Point<int16_t> text;
			// The guild's has "from '<name>'" first.
			bool from_first = false;
			// The alliance's has "'<name>'s" first instead.
			bool possessive = false;
		};

		// The invitations'.
		const Look LOOKS[UIInviteCard::ARRIVAL] = {
			{ "icon0", "An invitation", Point<int16_t>(24, 2) },
			{ "icon1", "Buddy request", Point<int16_t>(24, 2) },
			{ "icon2", "Trade request", Point<int16_t>(24, 2) },
			{ "icon5", "Party invite", Point<int16_t>(26, 4) },
			{ "icon5", "Guild invite", Point<int16_t>(26, 4), true },
			{ "icon5", "Guild Alliance Invitation.", Point<int16_t>(26, 4), true, true }
		};

		// v83 writes the arrivals at (25, 7), in Arial 11.
		const Point<int16_t> ARRIVAL_TEXT(24, 4);
		// And the parcels' at (27, 7), or one line at (27, 14), in Arial 12.
		const Point<int16_t> PARCEL_TEXT(26, 2), PARCEL_LINE(26, 9);

		// v83 draws on the notice's own window, which cuts off what runs
		// past its edge; here whole letters go.
		Text cut(const std::string& line, int16_t width)
		{
			Text text(Text::A12M, Text::LEFT, Color::Name::WHITE, line);
			std::string kept = line;

			while (!kept.empty() && text.width() > width)
			{
				kept.pop_back();
				text.change_text(kept);
			}

			return text;
		}
	}

	UIInviteCard::UIInviteCard() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0))
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		background = fade["backgrnd"];
		arrival_background = fade["backgrnd2"];
		parcel_background = fade["backgrnd4"];

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIInviteCard::draw(float alpha) const
	{
		for (const Notice& notice : notices)
		{
			Point<int16_t> at = position + offset(notice);
			float opacity = notice.life.opacity();
			const Texture& icon = notice.icon;

			const Texture& back = notice.kind == ARRIVAL ? arrival_background
				: notice.kind == PARCEL ? parcel_background : background;
			back.draw(DrawArgument(at, opacity));
			// v83 places an icon's top left; Duey's has an origin.
			icon.draw(DrawArgument(at + Point<int16_t>(ICON_X, (ICON_BAND - icon.get_dimensions().y()) / 2)
				+ icon.get_origin(), opacity));
			notice.first.draw(DrawArgument(at + notice.text, opacity));
			notice.second.draw(DrawArgument(at + notice.text + Point<int16_t>(0, LINE), opacity));
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

	void UIInviteCard::update()
	{
		bool changed = false;

		for (auto it = notices.begin(); it != notices.end();)
		{
			it->life.update(Constants::TIMESTEP);

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

		// Notices of other kinds coming and going move these too.
		if (changed || (!notices.empty() && notices.back().life.raised() != buttons_raised))
			rebuild();

		UIElement::update();
	}

	void UIInviteCard::add(Kind kind, const std::string& inviter, std::function<void(bool)> answer)
	{
		const Look& look = LOOKS[kind];
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];
		std::string from = look.possessive ? "'" + inviter + "'s" : "from '" + inviter + "'";

		push({
			kind,
			fade[look.icon],
			Text(Text::A12M, Text::LEFT, Color::Name::WHITE, look.from_first ? from : look.heading),
			Text(Text::A12M, Text::LEFT, Color::Name::WHITE, look.from_first ? look.heading : from),
			look.text,
			std::move(answer),
			FadeNotice(LIFETIME)
		});
	}

	void UIInviteCard::notify(const std::string& name, bool same_channel)
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];
		std::string first = same_channel ? "You're on the same ch." : name + " is";
		std::string second = same_channel ? "with " + name + "." : "logged in.";

		push({
			ARRIVAL,
			fade[same_channel ? "icon4" : "icon3"],
			Text(Text::A11M, Text::LEFT, Color::Name::WHITE, first),
			Text(Text::A11M, Text::LEFT, Color::Name::WHITE, second),
			ARRIVAL_TEXT,
			nullptr,
			FadeNotice(ARRIVAL_LIFETIME)
		});
	}

	void UIInviteCard::parcel(const std::string& sender, bool quick)
	{
		parcels::Arrival lines = parcels::arrival(sender, quick);
		Point<int16_t> text = lines.second.empty() ? PARCEL_LINE : PARCEL_TEXT;
		std::function<void(bool)> answer = [quick](bool yes)
		{
			if (yes && quick)
				DueyActionPacket(DueyActionPacket::Mode::OPEN).dispatch();
		};

		push({
			PARCEL,
			NxFiles::ui_classic["Basic.img"]["icon"]["delivery"],
			cut(lines.first, ARRIVAL_WIDTH - text.x()),
			cut(lines.second, ARRIVAL_WIDTH - text.x()),
			text,
			answer,
			FadeNotice(quick ? std::numeric_limits<int32_t>::max() : PARCEL_LIFETIME)
		});
	}

	void UIInviteCard::push(Notice notice)
	{
		Kind kind = notice.kind;

		if (kind != BUDDY)
			notices.erase(std::remove_if(notices.begin(), notices.end(), [kind](const Notice& n) { return n.kind == kind; }),
				notices.end());

		notices.push_back(std::move(notice));
		Sound(Sound::Name::INVITE).play();

		rebuild();
	}

	bool UIInviteCard::showing(Kind kind) const
	{
		return std::any_of(notices.begin(), notices.end(), [kind](const Notice& n) { return n.kind == kind; });
	}

	void UIInviteCard::rebuild()
	{
		buttons.clear();

		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		if (notices.empty())
			return;

		buttons_raised = notices.back().life.raised();
		Point<int16_t> at = offset(notices.back());

		if (notices.back().kind == ARRIVAL)
		{
			buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + ARRIVAL_CANCEL_AT);
		}
		else if (notices.back().kind == PARCEL)
		{
			buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], at + PARCEL_OK_AT);
			buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + PARCEL_CANCEL_AT);
		}
		else
		{
			buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], at + OK_AT);
			buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + CANCEL_AT);
		}
	}

	Point<int16_t> UIInviteCard::offset(const Notice& notice) const
	{
		return Point<int16_t>(0, -notice.life.raised());
	}

	void UIInviteCard::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(LEFT, new_height - ABOVE_BOTTOM);
	}

	bool UIInviteCard::is_in_range(Point<int16_t> cursorpos) const
	{
		if (notices.empty())
			return false;

		bool invitations = std::any_of(notices.begin(), notices.end(), [](const Notice& n) { return n.kind < ARRIVAL; });
		Point<int16_t> top = position + offset(notices.back());
		Point<int16_t> bottom = position + offset(notices.front()) + Point<int16_t>(invitations ? WIDTH : ARRIVAL_WIDTH, HEIGHT);

		return Rectangle<int16_t>(top, bottom).contains(cursorpos);
	}

	Button::State UIInviteCard::button_pressed(uint16_t buttonid)
	{
		// One press can arrive twice, and a notice fading out ignores more;
		// act on the first.
		if (notices.empty() || notices.back().life.closed())
			return Button::State::NORMAL;

		Notice& notice = notices.back();
		notice.life.close();

		// The answer may add a notice; let it work on its own copy.
		std::function<void(bool)> answer = notice.answer;

		if (answer)
			answer(buttonid == OK);

		return Button::State::NORMAL;
	}

	UIElement::Type UIInviteCard::get_type() const
	{
		return TYPE;
	}
}
