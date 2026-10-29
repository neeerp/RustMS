#pragma once

#include "GuildMakeTimeline.h"

#include "../UIElement.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"

#include <string>

namespace ms
{
	// The guild contract a party leader's members are asked to sign at
	// Heracle's (GUILD_OPERATION mode 3, v83 report b337, 0x592992). The
	// scroll unrolls, the contract fades in, and Agree or Enter signs it;
	// Disagree, Escape or thirty seconds refuse. The first answer stands.
	class UIGuildMakeAgreement : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GUILDMAKE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIGuildMakeAgreement(const std::string& master, const std::string& guild);

		void draw(float inter) const override;

		void update() override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void answer(bool agree);

		enum Buttons : uint16_t
		{
			BT_AGREE = 1,
			BT_DISAGREE = 2
		};

		Animation scroll;
		Texture message;
		Text master;
		Text guild;
		GuildMakeTimeline timeline;
		bool answered = false;
	};
}
