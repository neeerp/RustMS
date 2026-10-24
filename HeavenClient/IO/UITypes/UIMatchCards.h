#pragma once

#include "UIMiniGame.h"

#include "../../Graphics/Animation.h"

#include <nlnx/node.hpp>

namespace ms
{
	// A match cards room (v83 CMiniGameMatchDlg, 0x64d244): 12, 20 or 30
	// cards face down, turned up two a turn; a pair is the finder's, who
	// goes on.
	class UIMatchCards : public UIMiniGame
	{
	public:
		UIMatchCards(const mini_room::Enter &enter);

		void start(uint8_t waits, std::vector<int32_t> deck) override;

		void turn(uint8_t slot) override;

		void flipped_first(uint8_t index) override;

		void flipped_second(uint8_t index, uint8_t first, uint8_t code) override;

	protected:
		mini_game::Game &game() override;

		const mini_game::Game &game() const override;

		void draw_board(float inter) const override;

		void draw_counters(float inter) const override;

		bool tick() override;

		int32_t seconds_left() const override;

		void click_board(Point<int16_t> at) override;

		void hover_board(Point<int16_t> at, bool went_down) override;

		void leave_board() override;

		Rectangle<int16_t> board() const override;

		void refresh_buttons() override;

	private:
		// A pair's glow over one of its cards, face up until it fades
		// (0x65103f).
		struct Glow
		{
			Point<int16_t> at;
			int32_t face;
			Animation effect;
			int32_t ms;
		};

		void glow(uint8_t index, int32_t face);

		mini_game::Cards cards;
		Texture back;
		std::vector<Texture> faces;
		std::vector<Texture> numbers;
		nl::node effect;
		std::vector<Glow> glows;
		// What showed at the last tick, for the flip sound.
		std::vector<bool> was_up;
		// The seconds last shown, for the ticking.
		int32_t last_seconds;
		// The card the mouse went down on, drawn pressed in, or -1.
		int32_t pressed = -1;
	};
}
