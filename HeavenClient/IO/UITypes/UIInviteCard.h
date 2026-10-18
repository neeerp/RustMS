#pragma once

#include "../UIElement.h"
#include "../Components/FadeNotice.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// v83's fading invitations above the status bar (UIWindow.img/FadeYesNo,
	// FUN_0052065d): an invitation to Maple Messenger, a buddy request, a
	// trade request or a party invite, each "from '<name>'". O accepts and X
	// turns it down; one left alone fades out after three minutes and sends
	// nothing. A new one replaces any of its kind still there, except buddy
	// requests, which pile up (FUN_00a247cf). Newer notices stack above older
	// ones, quest completions included.
	class UIInviteCard : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::INVITECARD;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// v83's notice types 0, 1, 2 and 5.
		enum Kind : uint8_t { MESSENGER, BUDDY, TRADE, PARTY, NUM_KINDS };

		UIInviteCard();

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		// `answer` hears true for O and false for X.
		void add(Kind kind, const std::string& inviter, std::function<void(bool)> answer);
		// Whether a notice of `kind` is still there, even fading out.
		bool showing(Kind kind) const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CANCEL };

		struct Notice
		{
			Kind kind;
			Text from;
			std::function<void(bool)> answer;
			FadeNotice life;
		};

		void rebuild();
		// From the bottom of the stack to `notice`.
		Point<int16_t> offset(const Notice& notice) const;

		Texture background;
		std::array<Texture, NUM_KINDS> icons;
		std::array<Text, NUM_KINDS> headings;
		// Oldest first; the newest is drawn last, over the others.
		std::vector<Notice> notices;
		// How high the buttons sit, to move them when the stack shifts.
		int16_t buttons_raised = -1;

		// The window v83 makes for each (FUN_0051f9ad), on the same layer as
		// dialogs, so a dialog opened later covers it.
		static constexpr int16_t WIDTH = 196, HEIGHT = 44;
	};
}
