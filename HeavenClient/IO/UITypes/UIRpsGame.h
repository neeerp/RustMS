#pragma once

#include "../UIElement.h"

#include "../../Character/RpsGame.h"
#include "../../Character/Look/Face.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../Timer.h"

#include <array>

namespace ms
{
	// The Rock, Paper, Scissors window (v83 0x73d05b on UIWindow.img/RpsGame, report b038): the player's hand and
	// the admin's spinning one, Start, Continue or Retry in the middle, the three hands while choosing, and Exit.
	// The server plays every round. Exit is the only way out: Esc does nothing (report b041).
	class UIRpsGame : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::RPSGAME;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// StringPool 0xe5b, shown before the window.
		static constexpr const char* NOTICE =
			"#e#bRock, Paper, Scissors Notice#n#k\r\nThe participation fee for this Rock, Paper, Scissors game is #r1,000 mesos#k.\r\nWinning games will allow you to obtain various Winning Streak Certificates awarded for each consecutive win, but failure to complete the challenge will result in not receiving the Cerficate.\r\nThe received Certificate can be traded with the NPC's Paul, Jean, Martin, and Tony.";

		enum class Refusal : uint8_t
		{
			NO_MESOS,
			NO_SLOT,
			SILENT
		};

		UIRpsGame(int32_t npc_id);

		void draw(float alpha) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		UIElement::Type get_type() const override;

		// RPS_GAME 6, 7 and 14.
		void refused(Refusal refusal);
		// 9 and 12.
		void start_round();
		// 10.
		void time_over();
		// 11.
		void result(int8_t hand, int8_t streak);
		// 13.
		void close();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		int64_t now() const;
		void show_buttons();
		void play(RpsGame::Events events);

		enum Buttons : uint16_t
		{
			START,
			CONTINUE,
			RETRY,
			EXIT,
			ROCK,
			PAPER,
			SCISSORS
		};

		RpsGame game;
		ContinuousTimer::point opened;

		std::array<Texture, 3> own_hands;
		std::array<Texture, 3> npc_hands;
		// By RpsGame::Banner, NONE first.
		std::array<Texture, 5> banners;
		Animation npc;
		Expression::Id emotion;
		Text status;
		// Where the line starts in its 270 px.
		int16_t status_x;

		bool dragged;
		Point<int16_t> drag_from;
	};
}
