#pragma once

#include "UIMiniGame.h"

#include "../../Graphics/Animation.h"

namespace ms
{
	// An Omok room (v83 CMiniGameOmokDlg, 0x6e3124): the 15 by 15 board,
	// each player's stones, and a handicap to take a move back.
	class UIOmok : public UIMiniGame
	{
	public:
		UIOmok(const mini_room::Enter &enter);

		void start(uint8_t waits, std::vector<int32_t> deck) override;

		void turn(uint8_t slot) override;

		void undo_asked() override;

		void undo_result(bool undone, uint8_t count, uint8_t slot) override;

		void stone(const mini_game::Stone &stone) override;

		void rejected(uint8_t code) override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

		mini_game::Game &game() override;

		const mini_game::Game &game() const override;

		void draw_board(float inter) const override;

		void draw_counters(float inter) const override;

		bool tick() override;

		int32_t seconds_left() const override;

		void click_board(Point<int16_t> at) override;

		Rectangle<int16_t> board() const override;

	private:
		mini_game::Omok omok;
		// The set's stones, black and white.
		std::array<Animation, 2> stones;
	};
}
