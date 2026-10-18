#include "UIInviteCard.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Placed like the quest-complete notice (FUN_00523408).
		constexpr int16_t LEFT = 464, ABOVE_BOTTOM = 97;
		const Point<int16_t> OK_AT(174, 5), CANCEL_AT(174, 19);
		// The icon's left edge and the band it is centred in (FUN_0052065d).
		constexpr int16_t ICON_X = 6, ICON_BAND = 37;
		// How far the "from" line sits below the heading.
		constexpr int16_t LINE = 13;
		// How long from appearing until it fades out unanswered.
		constexpr uint32_t LIFETIME = 180000;

		struct Look
		{
			const char* icon;
			const char* heading;
			// v83's text position (FUN_0052065d), moved up to HC's text tops.
			Point<int16_t> text;
		};

		const Look LOOKS[UIInviteCard::NUM_KINDS] = {
			{ "icon0", "An invitation", Point<int16_t>(24, 2) },
			{ "icon1", "Buddy request", Point<int16_t>(24, 2) },
			{ "icon2", "Trade request", Point<int16_t>(24, 2) },
			{ "icon5", "Party invite", Point<int16_t>(26, 4) }
		};
	}

	UIInviteCard::UIInviteCard() : UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0))
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		background = fade["backgrnd"];

		for (size_t kind = 0; kind < NUM_KINDS; kind++)
		{
			icons[kind] = fade[LOOKS[kind].icon];
			headings[kind] = Text(Text::A12M, Text::LEFT, Color::Name::WHITE, LOOKS[kind].heading);
		}

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIInviteCard::draw(float alpha) const
	{
		for (const Notice& notice : notices)
		{
			Point<int16_t> at = position + offset(notice);
			float opacity = notice.life.opacity();
			const Texture& icon = icons[notice.kind];
			Point<int16_t> text = LOOKS[notice.kind].text;

			background.draw(DrawArgument(at, opacity));
			icon.draw(DrawArgument(at + Point<int16_t>(ICON_X, (ICON_BAND - icon.get_dimensions().y()) / 2), opacity));
			headings[notice.kind].draw(DrawArgument(at + text, opacity));
			notice.from.draw(DrawArgument(at + text + Point<int16_t>(0, LINE), opacity));
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
		if (kind != BUDDY)
			notices.erase(std::remove_if(notices.begin(), notices.end(), [kind](const Notice& n) { return n.kind == kind; }),
				notices.end());

		Text from(Text::A12M, Text::LEFT, Color::Name::WHITE, "from '" + inviter + "'");

		notices.push_back({ kind, std::move(from), std::move(answer), FadeNotice(LIFETIME) });
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

		buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], at + OK_AT);
		buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + CANCEL_AT);
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

		Point<int16_t> top = position + offset(notices.back());
		Point<int16_t> bottom = position + offset(notices.front()) + Point<int16_t>(WIDTH, HEIGHT);

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
		answer(buttonid == OK);

		return Button::State::NORMAL;
	}

	UIElement::Type UIInviteCard::get_type() const
	{
		return TYPE;
	}
}
