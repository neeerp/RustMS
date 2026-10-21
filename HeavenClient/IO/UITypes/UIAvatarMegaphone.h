#pragma once

#include "../UIElement.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../Util/Megaphones.h"

#include <array>
#include <string>

namespace ms
{
	// An avatar megaphone's banner (v83 FUN_0045a4cd, drawn by FUN_0045aef1): the item's art in the top right
	// corner, the speaker standing in it with the item's face, the name on a plate under them, and four lines in
	// the speech box. It slides in from the right edge with a wobble (FUN_0045b2b9), and back out after 10 s or
	// when the server clears it (FUN_0045b818). The next banner replaces it at once. Clicks go through it.
	class UIAvatarMegaphone : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::AVATARMEGAPHONE;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIAvatarMegaphone(int32_t item_id, const std::string &name, const Megaphones::AvatarLines &lines,
						  const LookEntry &look);

		void draw(float alpha) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		// Slide out, as CLEAR_AVATAR_MEGAPHONE asks.
		void hide();

	private:
		// How far right of its place the banner is drawn.
		int16_t offset() const;

		Animation art;
		Texture plate;
		int16_t plate_x;
		Text name;
		std::array<Text, Megaphones::AVATAR_LINES> lines;
		CharLook look;
		Expression::Id expression;
		// Since it came in, and since it began to leave, in ms.
		int32_t shown;
		int32_t leaving;
		// How far right it was when it began to leave.
		int16_t left_at;
	};
}
