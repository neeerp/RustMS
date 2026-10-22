#pragma once

#include "../UIElement.h"

#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>

namespace ms
{
	// A guest's wedding invitation, as MARRIAGE_RESULT 15 opens it (v83
	// 7ec9c1, report b069): the venue's card with the groom's name left of
	// the "&" and the bride's right of it. OK, Enter or Escape close it;
	// nothing is sent.
	class UIInvitation : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::INVITATION;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `type` 2 is the Cathedral's card, anything else the Chapel's.
		UIInvitation(const std::string& groom, const std::string& bride, int16_t type);

		void draw(float inter) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			BT_OK
		};

		Texture background;
		Text groom;
		Text bride;
	};
}
