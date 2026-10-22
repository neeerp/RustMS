#pragma once

#include "../UIDragElement.h"

#include "../Components/Textfield.h"

#include "../../Character/PartySearch.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>

namespace ms
{
	// v83's Party Search window (UIWindow.img/PartySearch, 0x87781d,
	// reports b034 and b036): the levels, party size and jobs a leader
	// looks for, and Start, Pause and Stop. The search itself lives in
	// PartySearch, so it goes on with the window closed.
	class UIPartySearch : public UIDragElement<PosPARTYSEARCH>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::PARTYSEARCH;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIPartySearch();

		// Open or close it, as BtSearch, the hotkey and /partysearch do
		// (0xa04f7a). It opens beside the user list when that is open.
		static void toggle();

		void draw(float inter) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The search changed: enable what its state allows (0x8795e4), and
		// show the defaults again after a stop.
		void refresh();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { START, PAUSE, STOP };

		// What the window shows when it opens (0x879741): the search, or
		// the defaults without one.
		void load();
		void show(const PartySearch::Wanted& wanted);
		void start();

		// The box under the cursor, or BOXES.
		size_t box_at(Point<int16_t> cursorpos) const;

		Texture background;
		std::array<Texture, 5> layout;
		std::array<Texture, 4> checkbox;
		ColorBox field_fill;
		std::array<Textfield, 3> fields;
		std::array<Text, PartySearch::BOXES> labels;
		std::array<bool, PartySearch::BOXES> shown;
		int32_t jobs = 0;
		bool enabled = true;
		size_t pressed = PartySearch::BOXES;
		PartySearch::State seen = PartySearch::State::IDLE;
	};
}
