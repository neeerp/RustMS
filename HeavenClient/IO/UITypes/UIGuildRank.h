#pragma once

#include "../UIElement.h"

#include "../../Character/Guild.h"
#include "../../Character/GuildMark.h"
#include "../../Graphics/Text.h"

#include <vector>

namespace ms
{
	// The guild ranking that the Honorable Rock and the Guild Rank Board open (v83 FUN_005958c0 on
	// UIWindow.img/UserList/Guild/GuildRank, report b181): the leader under the crown, then six guilds a page. It is
	// modal; OK, Enter and Escape close it, and nothing is sent.
	class UIGuildRank : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GUILDRANK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `own` is the viewer's guild's emblem, whose ids fill a row's zero ones.
		UIGuildRank(std::vector<GuildStanding> guilds, const GuildEmblem &own);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void turn_to(int32_t page);

		void draw_row(int32_t rank, int16_t y) const;

		enum Buttons : uint16_t
		{
			OK,
			LEFT,
			RIGHT
		};

		struct Row
		{
			Text rank;
			GuildMark emblem;
			Text name;
			Text points;
		};

		std::vector<Row> rows;
		int32_t page;
	};
}
