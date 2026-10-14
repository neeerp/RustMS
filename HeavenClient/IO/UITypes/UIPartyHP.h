#pragma once

#include "../UIElement.h"

#include "../../Character/CharStats.h"
#include "../../Character/Party.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// The v83 party HP list (UIWindow.img/UserList/Party/PartyHP): every
	// member's name and HP gauge, near the top-right corner of the screen.
	// Members off the player's map are greyed out with empty gauges.
	class UIPartyHP : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PARTYHP;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIPartyHP(const Party& party, const CharStats& stats);

		void draw(float inter) const override;
		void update() override;
		void update_screen(int16_t new_width, int16_t new_height) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		// How far down the screen the list reaches, or 0 without a party.
		static int16_t height_for(const Party& party);

	private:
		struct Row
		{
			int32_t id;
			std::string name;
			bool nearby;
			Text label;
		};

		const Party& party;
		const CharStats& stats;
		std::vector<Row> rows;
		Texture nw, ne, w, e, sw, se;
		Texture graduation, bar, gauge;

		// The box is wider than its content by the left border.
		static constexpr int16_t WIDTH = 149, BORDER = 5, EDGE = 7;
		// From footage: rows 17 px apart, gauges flush right with a 5 px
		// margin, and the box 24 px from the right of the screen.
		// Names are centred on their gauges.
		static constexpr int16_t ROW = 17, GAUGE_X = 70, GAUGE_Y = -2, NAME_Y = -9, RIGHT_MARGIN = 24;
		static constexpr int16_t GAUGE_WIDTH = 63;
	};
}
