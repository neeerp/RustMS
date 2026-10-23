#include "../Net/Handlers/MiniGame.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::mini_game;

		require(is_omok_set(4080000) && is_omok_set(4080011) && !is_omok_set(4080100), "Omok sets are 40800xx");
		require(is_game_set(4080100) && !is_game_set(4090000), "the match cards are a game set too");

		require(message_lines(7, "Bob") == std::vector<std::string>{ "[Bob] have entered." }, "a notice names its player");
		require(message_lines(0x67, "").size() == 2, "the game's end takes two lines");
		require(message_about_player(9) && !message_about_player(0x66), "the game's own notices are told apart");
		require(message_lines(0x50, "Bob").empty(), "an unknown notice says nothing");

		require(leave_text(5) == "You have been expelled." && leave_text(0) == leave_text(4), "the ways out");
		require(!leave_closes(2) && leave_closes(3), "the opponent leaving keeps the room");

		// Cells are 14 px squares every 24 px from (25, 32).
		require(cell_at(25, 32) == std::make_pair(0, 0), "the first cell's corner");
		require(cell_at(38, 45) == std::make_pair(0, 0), "and its far corner");
		require(!cell_at(39, 45) && !cell_at(24, 32), "the gaps hit nothing");
		require(cell_at(25 + 24 * 14, 32 + 24 * 7) == std::make_pair(14, 7), "the last column");
		require(!cell_at(25 + 24 * 15, 40), "and nothing past it");
		require(stone_position(1, 2) == std::make_pair<int16_t, int16_t>(45, 76), "stones sit on their cells");

		// Alice owns the room and waits first, so Bob opens with black.
		Omok alice(0), bob(1);
		alice.start(0);
		bob.start(0);
		require(!alice.my_turn && alice.my_color == 2 && bob.my_turn && bob.my_color == 1, "whoever waits has white");
		require(bob.can_place(3, 3) && !alice.can_place(3, 3), "only the mover may place");
		require(bob.can_ask_undo() == Omok::Undo::NO, "nothing to take back yet");

		for (Omok *game : { &alice, &bob })
			game->place({ 3, 3, 1 });

		require(alice.my_turn && !bob.my_turn && !alice.can_place(3, 3), "the turn passes, and the cell is taken");
		require(bob.can_ask_undo() == Omok::Undo::ASK, "Bob may take his stone back");
		bob.ask_undo();
		require(bob.can_ask_undo() == Omok::Undo::NO, "once at a time");
		bob.undo(1, 1);
		require(bob.stones.empty() && bob.my_turn, "his stone is gone and he moves again");
		bob.place({ 4, 4, 1 });
		require(bob.can_ask_undo() == Omok::Undo::USED, "he had his undo this game");

		require(alice.can_ask_tie(), "a tie may be asked");
		alice.ask_tie();
		alice.tie_denied();
		require(!alice.can_ask_tie(), "once a turn");
		alice.place({ 3, 3, 1 });
		require(alice.can_ask_tie(), "and again next turn");

		// The timer runs out once, for whoever moves.
		bob.turn(1);
		require(!bob.tick(29000) && bob.seconds_left() == 1, "a second left");
		require(bob.tick(1000) && !bob.tick(30), "time runs out once");
		alice.turn(1);
		require(!alice.tick(30000), "not for whoever waits");
		bob.turn(0);
		require(bob.seconds_left() == 30 && !bob.my_turn, "a new turn starts the clock again");

		bob.end(false, 1);
		require(bob.ended() && bob.won() && !bob.can_place(0, 0), "Bob won");
		bob.settle();
		require(!bob.ended() && !bob.playing(), "then the room waits");

		// The card grids (0x64dbac): 4 x 3 from (89, 106), 6 x 5 from (33, 39).
		require(card_grid(0).count == 12 && card_grid(1).count == 20 && card_grid(2).count == 30, "three sizes");
		require(card_position(0, 5) == std::make_pair<int16_t, int16_t>(89 + 56, 106 + 67), "row by row");
		require(card_at(2, 33, 39) == 0 && card_at(2, 33 + 48, 39 + 61) == 0, "a card's corners");
		require(card_at(2, 33 + 49, 39) == -1 && card_at(2, 33 + 56, 39) == 1, "the gaps hit nothing");
		require(card_at(0, 33, 39) == -1, "a smaller grid starts further in");

		// Alice waits, so Bob opens; each card shows at the start.
		Cards cards_a(0, 0), cards_b(1, 0);
		std::vector<int32_t> deck = { 0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5 };
		cards_a.start(0, deck);
		cards_b.start(0, deck);
		require(cards_b.my_turn && !cards_a.my_turn, "whoever waits moves second");
		require(!cards_b.can_flip(0), "not while the cards show");
		cards_b.tick(CARD_REVEAL_MS);
		require(cards_b.face_up(0) && cards_b.face_up(1) && !cards_b.face_up(2), "one more every 200 ms");
		cards_b.tick(CARD_REVEAL_MS * 4);
		require(!cards_b.face_up(0) && cards_b.face_up(5) && !cards_b.face_up(6), "each for a second");
		require(!cards_b.tick(12 * CARD_REVEAL_MS), "the time is not out yet");
		require(!cards_b.face_up(11) && !cards_b.can_flip(0), "the show is over, and the wait not yet");
		cards_b.tick(500);
		require(cards_b.can_flip(0) && cards_b.seconds_left() == 10, "then Bob may pick");

		require(cards_b.flip(0) && cards_b.face_up(0) && !cards_b.can_flip(0), "the first card shows at once");
		require(!cards_b.flip(6) && !cards_b.face_up(6) && !cards_b.can_flip(1), "the second waits for the server");
		require(cards_b.flipped_second(6, 0, 1 + 2), "a pair");
		require(cards_b.removed(0) && cards_b.removed(6) && cards_b.pairs[1] == 1 && cards_b.my_turn, "Bob goes on");
		cards_a.tick(20000);
		cards_a.flipped_first(0);
		cards_a.flipped_second(6, 0, 1 + 2);
		require(cards_a.pairs[1] == 1 && !cards_a.my_turn, "Alice sees it too");

		cards_b.flip(1);
		require(!cards_b.flipped_second(2, 1, 1), "a miss");
		require(!cards_b.my_turn && cards_b.face_up(1) && cards_b.face_up(2), "both show, and Bob's turn is over");
		require(cards_b.seconds_left() == 10, "a miss's longer turn shows 10");
		cards_b.tick(CARD_HIDE_MS);
		require(!cards_b.face_up(1) && !cards_b.face_up(2), "then turn back");
		cards_a.flipped_second(2, 1, 1);
		require(cards_a.my_turn && !cards_a.can_flip(3), "Alice's turn, after a pause");
		cards_a.tick(CARD_MISS_MS - CARD_TURN_MS);
		require(cards_a.can_flip(3), "and then she picks");
		cards_a.flip(3);
		require(cards_a.tick(CARD_TURN_MS) && !cards_a.tick(8), "her time runs out once");
		cards_a.turn(1);
		require(!cards_a.face_up(3) && !cards_a.my_turn, "her card goes back as the turn passes");

		cards_a.end(false, 1);
		require(cards_a.ended() && !cards_a.won() && !cards_a.can_flip(4), "Bob won");

		std::cout << "Mini game passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
