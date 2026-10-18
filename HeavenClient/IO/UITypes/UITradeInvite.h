#pragma once

#include "../UIElement.h"
#include "../Components/FadeNotice.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// v83's fading "Trade request" notice (UIWindow.img/FadeYesNo/backgrnd,
	// FUN_00522f14) above the status bar, shown when someone invites us to
	// trade. O joins their trading room; X, or leaving it until it fades out,
	// turns the invite down. Newer notices stack above older ones.
	class UITradeInvite : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::TRADEINVITE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UITradeInvite();

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		void add(const std::string& inviter, int32_t serial);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CANCEL };

		struct Notice
		{
			int32_t serial;
			Text from;
			FadeNotice life;
			// Pressed, or turned down when it ran out.
			bool answered;
		};

		void rebuild();
		// From the first notice to the one at `index`.
		Point<int16_t> offset(size_t index) const;

		Texture background, icon;
		Text heading;
		// Oldest first; the newest is drawn last, over the others.
		std::vector<Notice> notices;

		static constexpr int16_t WIDTH = 195, HEIGHT = 44;
	};
}
