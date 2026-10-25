#pragma once

#include "../UIDragElement.h"

#include "../Components/Slider.h"
#include "../Components/TextTooltip.h"

#include "../../Character/Carnival.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <cstdint>
#include <vector>

namespace ms
{
	// The Monster Carnival's CP window (CUIMonsterCarnival, b166): the CP
	// counts, and the field's mobs, skills and guardians to spend them on.
	// START on a battle field puts it up; leaving the field takes it down.
	class UIMonsterCarnival : public UIDragElement<PosMONSTERCARNIVAL>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MONSTERCARNIVAL;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIMonsterCarnival();

		// Puts the window up for the field being entered, if it isn't yet.
		static void show();

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void doubleclick(Point<int16_t> cursorpos) override;

		// Tab and F1 to F12, which the window takes while it is up (A
		// a12355). Whether it took the key.
		bool send_hotkey(int32_t keycode, bool pressed);

		// A mob summoned, by its name, for the side panel (FUN_0086df54).
		void summoned(const std::string& name);

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// A row's texts in black, and in white for when it is selected.
		struct Row
		{
			std::array<Text, 2> name;
			std::array<Text, 2> cost;
			std::array<Text, 2> key;
			int16_t cost_x;
			std::string label;
			std::string desc;
		};

		// A side panel row's texts, the same way.
		struct Count
		{
			std::array<Text, 2> name;
			std::array<Text, 2> count;
			bool latest;
		};

		void add_row(uint8_t tab, const std::string& name, int32_t cost, const std::string& desc);
		void relayout();
		void refresh_points();
		void select_tab(uint8_t tab);
		// Spends CP on the selected row (FUN_008706d3).
		void spend() const;
		void show_tip(Point<int16_t> relative);
		// The side panel's texts and bar, after a change (FUN_0086d6ae).
		void refresh_side();
		// Whether `relative` is on the side panel.
		bool on_side(Point<int16_t> relative) const;
		bool on_button(Point<int16_t> cursorpos) const;

		enum Buttons : uint16_t
		{
			BT_MAX = 2,
			BT_MIN = 3,
			BT_SIDE = 4,
			BT_DECIDE = 5
		};

		std::array<std::vector<Row>, carnival::window::TABS> rows;
		carnival::window::State state;

		Texture background, top, header, middle, bottom;
		std::array<Texture, 2> team_icons;
		std::array<Texture, 2> lock_icons;
		std::array<Texture, carnival::window::TABS> tab_on, tab_off;

		Text my_cp, our_cp, their_cp;
		carnival::Points shown_mine;
		std::array<carnival::Points, 2> shown_sides;
		int8_t shown_team;

		Slider slider;

		carnival::window::Tally tally;
		std::vector<Count> counts;
		Texture side_background;
		Slider side_slider;
		bool side_open;
		TextTooltip tip;
		Point<int16_t> cursor;
		bool hovered;
	};
}
