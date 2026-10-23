#include "UIMatchCards.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where match cards' Draw puts the shared things, a pixel or so off
		// Omok's (0x64f358, b100 §2.1).
		const UIMiniGame::Layout LAYOUT = {
			Point<int16_t>(507, 20),
			{ 169, 187, 201, 215 },
			{ 452, 538 },
			{ 465, 551 },
			Point<int16_t>(490, 167),
			Point<int16_t>(13, 404),
			Point<int16_t>(295, 404)
		};

		// Each player's pairs on their record panel (0x652078).
		constexpr int16_t PAIRS_X[2] = { 411, 497 };
		constexpr int16_t PAIRS_Y = 207;

		// The cards lie on the board's part of the window.
		const Rectangle<int16_t> BOARD = Rectangle<int16_t>(11, 386, 19, 389);

		// The faces and pair counts the art has.
		constexpr int32_t FACES = 15;
		constexpr int32_t NUMBERS = 16;

		// The art is one image; the client shows it as 5 frames of 300 ms (65103f).
		constexpr int32_t GLOW_MS = 5 * 300;

		nl::node memory_node()
		{
			return NxFiles::ui_classic["UIWindow.img"]["Minigame"]["MemoryGame"];
		}
	}

	UIMatchCards::UIMatchCards(const mini_room::Enter &enter) :
			UIMiniGame(enter, memory_node()["backgrnd"], LAYOUT),
			cards(enter.own_slot < 2 ? enter.own_slot : 0, enter.piece), last_seconds(0)
	{
		nl::node MemoryGame = memory_node();

		back = MemoryGame["card"]["back" + std::to_string(cards.size)];

		for (int32_t i = 0; i < FACES; i++)
			faces.emplace_back(MemoryGame["card"][std::to_string(i)]);

		for (int32_t i = 0; i < NUMBERS; i++)
			numbers.emplace_back(MemoryGame["number"][std::to_string(i)]);

		effect = MemoryGame["effect" + std::to_string(cards.size)];

		refresh_buttons();
	}

	void UIMatchCards::start(uint8_t waits, std::vector<int32_t> deck)
	{
		cards.start(waits, std::move(deck));
		was_up.assign(cards.deck.size(), false);
		glows.clear();
		last_seconds = 0;

		refresh_buttons();
	}

	void UIMatchCards::turn(uint8_t slot)
	{
		cards.turn(slot);
	}

	void UIMatchCards::flipped_first(uint8_t index)
	{
		cards.flipped_first(index);
	}

	void UIMatchCards::flipped_second(uint8_t index, uint8_t first, uint8_t code)
	{
		int32_t faces_of[2] = {
			cards.removed(first) ? -1 : cards.deck[first],
			cards.removed(index) ? -1 : cards.deck[index]
		};

		if (cards.flipped_second(index, first, code))
		{
			// The second card shows before the pair goes (164 §5).
			glow(first, faces_of[0]);
			glow(index, faces_of[1]);
			play("Open");
			play("Match");
		}
		else
		{
			play("NoMatch");
		}
	}

	mini_game::Game &UIMatchCards::game()
	{
		return cards;
	}

	const mini_game::Game &UIMatchCards::game() const
	{
		return cards;
	}

	void UIMatchCards::draw_board(float inter) const
	{
		for (int32_t i = 0; i < static_cast<int32_t>(cards.deck.size()); i++)
		{
			if (cards.removed(i))
				continue;

			auto at = mini_game::card_position(cards.size, i);
			Point<int16_t> place = position + Point<int16_t>(at.first, at.second);
			int32_t value = cards.deck[i];

			if (cards.face_up(i) && value < FACES)
				faces[value].draw(place);
			else
				back.draw(place);
		}

		for (const Glow &glow : glows)
		{
			if (glow.face >= 0 && glow.face < FACES)
				faces[glow.face].draw(position + glow.at);

			glow.effect.draw(position + glow.at, inter);
		}
	}

	void UIMatchCards::draw_counters(float) const
	{
		if (!cards.playing() && !cards.ended())
			return;

		for (uint8_t slot = 0; slot < 2; slot++)
			if (cards.pairs[slot] < NUMBERS)
				numbers[cards.pairs[slot]].draw(position + Point<int16_t>(PAIRS_X[slot], PAIRS_Y));
	}

	bool UIMatchCards::tick()
	{
		for (Glow &glow : glows)
		{
			glow.effect.update();
			glow.ms -= Constants::TIMESTEP;
		}

		glows.erase(std::remove_if(glows.begin(), glows.end(), [](const Glow &glow) { return glow.ms <= 0; }), glows.end());

		bool timed_out = cards.tick(Constants::TIMESTEP);

		// A card turning up makes a sound, the start's show too (0x653ae5).
		bool opened = false;

		for (size_t i = 0; i < was_up.size(); i++)
		{
			bool up = cards.face_up(static_cast<int32_t>(i));

			opened = opened || (up && !was_up[i]);
			was_up[i] = up;
		}

		if (opened)
			play("Open");

		// My last seconds tick away (0x64efcc).
		int32_t seconds = cards.playing() && cards.my_turn ? cards.seconds_left() : 0;

		if (seconds != last_seconds && seconds >= 1 && seconds <= 9)
			play("Timer");

		last_seconds = seconds;

		return timed_out;
	}

	int32_t UIMatchCards::seconds_left() const
	{
		return cards.seconds_left();
	}

	void UIMatchCards::click_board(Point<int16_t> at)
	{
		// A card goes up on my turn: the first at once, the second when the
		// server says how it went (0x64ece1).
		int32_t index = mini_game::card_at(cards.size, at.x(), at.y());

		if (!cards.can_flip(index))
			return;

		bool first = cards.flip(index);

		MatchCardsFlipPacket(first, static_cast<uint8_t>(index)).dispatch();
	}

	Rectangle<int16_t> UIMatchCards::board() const
	{
		return BOARD;
	}

	void UIMatchCards::refresh_buttons()
	{
		UIMiniGame::refresh_buttons();

		// A tie and giving up are for a game under way (164 §5).
		Button::State state = cards.playing() ? Button::State::NORMAL : Button::State::DISABLED;

		buttons[Buttons::TIE]->set_state(state);
		buttons[Buttons::GIVE_UP]->set_state(state);
	}

	void UIMatchCards::glow(uint8_t index, int32_t face)
	{
		auto at = mini_game::card_position(cards.size, index);

		glows.push_back({ Point<int16_t>(at.first, at.second), face, Animation(effect), GLOW_MS });
	}
}
