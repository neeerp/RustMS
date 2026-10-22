#pragma once

#include "../UIElement.h"
#include "../Components/FadeNotice.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// v83's fading invitations above the status bar (UIWindow.img/FadeYesNo,
	// FUN_0052065d): an invitation to Maple Messenger, a buddy request, a
	// trade request, a party invite or a guild invite, each "from '<name>'",
	// or an alliance's, "'<name>'s Guild Alliance Invitation.". O accepts and X
	// turns it down; one left alone fades out after three minutes and sends
	// nothing. Smaller pink ones say a buddy logged in or came to the
	// player's channel, for six seconds, with only an X. Duey's say a parcel
	// came. A new one replaces any of its kind still there, except buddy
	// requests, which pile up (FUN_00a247cf). Newer notices stack above
	// older ones, quest completions included.
	class UIInviteCard : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::INVITECARD;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// v83's notice types 0, 1, 2, 5, 8, 6, 9 and 10.
		enum Kind : uint8_t { MESSENGER, BUDDY, TRADE, PARTY, GUILD, ALLIANCE, ARRIVAL, PARCEL, NUM_KINDS };

		UIInviteCard();

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		// `answer` hears true for O and false for X.
		void add(Kind kind, const std::string& inviter, std::function<void(bool)> answer);
		// A buddy logged in, or came to the player's channel (FUN_00523a2c).
		void notify(const std::string& name, bool same_channel);
		// A parcel came (PARCEL modes 0x19 and 0x1b, FUN_00523c60), from
		// `sender` if named. A plain one's goes in six seconds; O on a
		// quick one's opens the parcels.
		void parcel(const std::string& sender, bool quick);
		// Whether a notice of `kind` is still there, even fading out.
		bool showing(Kind kind) const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CANCEL };

		struct Notice
		{
			Kind kind;
			Texture icon;
			Text first, second;
			// Where the first line sits.
			Point<int16_t> text;
			// Empty for the arrivals, which ask nothing.
			std::function<void(bool)> answer;
			FadeNotice life;
		};

		void push(Notice notice);

		void rebuild();
		// From the bottom of the stack to `notice`.
		Point<int16_t> offset(const Notice& notice) const;

		Texture background, arrival_background, parcel_background;
		// Oldest first; the newest is drawn last, over the others.
		std::vector<Notice> notices;
		// How high the buttons sit, to move them when the stack shifts.
		int16_t buttons_raised = -1;

		// The window v83 makes for each (FUN_0051f9ad), on the same layer as
		// dialogs, so a dialog opened later covers it.
		static constexpr int16_t WIDTH = 196, ARRIVAL_WIDTH = 155, HEIGHT = 44;
	};
}
