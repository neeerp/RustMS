#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// The parts of v83's Omok and match cards (CMiniGameDlg and its rooms,
	// report 164 §3-5, b100) that need no UI.
	namespace mini_game
	{
		// A locked game's password, at most (0x7dc8ef).
		constexpr size_t PASSWORD_LIMIT = 12;
		// A room's title, at most.
		constexpr size_t TITLE_LIMIT = 30;

		// The game sets in the Etc bag: 4080000 to 4080011 are Omok's, each
		// with its own stones, 4080100 the match cards (0x4eff54).
		inline bool is_game_set(int32_t itemid)
		{
			return itemid / 10000 == 408;
		}

		inline bool is_omok_set(int32_t itemid)
		{
			return is_game_set(itemid) && (itemid / 100) % 10 == 0;
		}

		// The game's notice `id` about `name` in the room's chat (mode 6
		// sub 7, 0x65f79e), line by line.
		inline std::vector<std::string> message_lines(uint8_t id, const std::string &name)
		{
			std::string text;

			switch (id)
			{
			case 0:
				text = "[%s] have been expelled.";
				break;
			case 1:
				text = "[%s]'s turn.";
				break;
			case 2:
				text = "[%s] have forfeited.";
				break;
			case 3:
				text = "[%s] have requested a handicap.";
				break;
			case 4:
				text = "[%s] have left.";
				break;
			case 5:
				text = "[%s] have called to leave after this game.";
				break;
			case 6:
				text = "[%s] have cancelled the request to leave after this game.";
				break;
			case 7:
				text = "[%s] have entered.";
				break;
			case 8:
				text = "[%s] can't start the game due to lack of mesos.";
				break;
			case 9:
				text = "[%s] has matched cards. Please continue.";
				break;
			case 0x65:
				return { "10 sec. left." };
			case 0x66:
				return { "The game has started." };
			case 0x67:
				return { "The game has ended.", "The room will automatically close in 10 sec." };
			default:
				return {};
			}

			text.replace(text.find("%s"), 2, name);

			return { text };
		}

		// The notices about a player are yellow, the game's own red.
		inline bool message_about_player(uint8_t id)
		{
			return id < 0x65;
		}

		// Why we left the room (0x6e3d06); reason 2, the opponent leaving,
		// leaves it open.
		inline std::string leave_text(int8_t reason)
		{
			switch (reason)
			{
			case 0:
			case 4:
				return "You have left the room.";
			case 3:
				return "The room is closed.";
			case 5:
				return "You have been expelled.";
			default:
				return "";
			}
		}

		inline bool leave_closes(int8_t reason)
		{
			return reason != 2;
		}

		// The Omok board: 15 cells a side, each a 14 px square every 24 px
		// from (25, 32); a point between them is on none (0x6e8f91).
		constexpr int32_t BOARD_CELLS = 15;
		constexpr int16_t CELL_LEFT = 25;
		constexpr int16_t CELL_TOP = 32;
		constexpr int16_t CELL_PITCH = 24;
		constexpr int16_t CELL_SIZE = 14;

		inline std::optional<std::pair<int32_t, int32_t>> cell_at(int16_t x, int16_t y)
		{
			auto axis = [](int16_t at, int16_t start) -> int32_t
			{
				int32_t offset = at - start;

				if (offset < 0 || offset % CELL_PITCH >= CELL_SIZE || offset / CELL_PITCH >= BOARD_CELLS)
					return -1;

				return offset / CELL_PITCH;
			};

			int32_t cx = axis(x, CELL_LEFT);
			int32_t cy = axis(y, CELL_TOP);

			if (cx < 0 || cy < 0)
				return std::nullopt;

			return std::make_pair(cx, cy);
		}

		// Where a stone on (x, y) is drawn (0x6e7ca6).
		inline std::pair<int16_t, int16_t> stone_position(int32_t x, int32_t y)
		{
			return { static_cast<int16_t>(CELL_PITCH * x + 21), static_cast<int16_t>(CELL_PITCH * y + 28) };
		}

		// A turn lasts 30 seconds (0x6e469c).
		constexpr int32_t TURN_MS = 30000;

		struct Stone
		{
			int32_t x;
			int32_t y;
			// 1 black, who moves first; 2 white.
			uint8_t color;
		};

		// What Omok and match cards share, as one player sees it: whose turn
		// it is, the requests of a game and its result (CMiniGameDlg).
		class Game
		{
		public:
			explicit Game(uint8_t own_slot) : own_slot(own_slot) {}

			virtual ~Game() = default;

			// A tie may be asked once a turn, one at a time (0x6e8b24).
			bool can_ask_tie() const
			{
				return state == State::PLAYING && !tie_asked && !tie_this_turn;
			}

			void ask_tie()
			{
				tie_asked = true;
				tie_this_turn = true;
			}

			void tie_denied()
			{
				tie_asked = false;
			}

			bool can_give_up() const
			{
				return state == State::PLAYING && !gave_up;
			}

			void give_up()
			{
				gave_up = true;
			}

			// Exit while playing asks to leave after the game, or no longer
			// (0x6e8e29).
			void toggle_leave_after()
			{
				leave_after = !leave_after;
			}

			// The game ended (0x3e): a tie, or won by the player in
			// `winner_slot`.
			virtual void end(bool is_tie, uint8_t winner_slot)
			{
				state = State::ENDED;
				tie = is_tie;
				winner = winner_slot;
				ready = false;
			}

			// The result has been shown; the room waits for the next game.
			void settle()
			{
				if (state == State::ENDED)
					state = State::WAITING;
			}

			bool playing() const
			{
				return state == State::PLAYING;
			}

			bool ended() const
			{
				return state == State::ENDED;
			}

			bool won() const
			{
				return !tie && winner == own_slot;
			}

			uint8_t turn_slot() const
			{
				return my_turn ? own_slot : static_cast<uint8_t>(1 - own_slot);
			}

			uint8_t own_slot;
			bool my_turn = false;
			// The visitor said they are ready (0x3a, 0x3b).
			bool ready = false;
			bool leave_after = false;
			bool tie = false;
			uint8_t winner = 0;

		protected:
			enum class State
			{
				WAITING,
				PLAYING,
				ENDED
			};

			// The game starts; the player in `waits` moves second (0x3d).
			void begin(uint8_t waits)
			{
				state = State::PLAYING;
				my_turn = waits != own_slot;
				ready = false;
				gave_up = false;
				leave_after = false;
				tie_asked = false;
				tie_this_turn = false;
				tie = false;
				winner = 0;
			}

			// A new turn: the server drops whatever was asked in the last
			// one, so an answer to it does nothing (field_games give_turn).
			void new_turn()
			{
				tie_asked = false;
				tie_this_turn = false;
			}

			State state = State::WAITING;
			bool gave_up = false;
			bool tie_asked = false;
			bool tie_this_turn = false;
		};

		// An Omok game as one player sees it (CMiniGameOmokDlg's fields
		// +0xad0 to +0xb14).
		class Omok : public Game
		{
		public:
			enum class Undo
			{
				ASK,
				// Not now: nothing to take back, or already asked.
				NO,
				// Once a game (SP 0x1cd).
				USED
			};

			explicit Omok(uint8_t own_slot) : Game(own_slot) {}

			// The game starts; the player in `waits` moves second, with
			// white (0x3d).
			void start(uint8_t waits)
			{
				begin(waits);
				stones.clear();
				my_color = my_turn ? 1 : 2;
				my_stones = 0;
				timer = TURN_MS;
				timed_out = false;
				undo_asked = false;
				undo_this_turn = false;
				undo_used = false;
			}

			// A stone went down (0x40); whoever did not put it moves next.
			void place(const Stone &stone)
			{
				stones.push_back(stone);

				if (stone.color == my_color)
				{
					my_stones++;
					my_turn = false;
				}
				else
				{
					my_turn = true;
				}

				new_turn();
				undo_asked = false;
				undo_this_turn = false;
				restart_timer();
			}

			// The last `count` stones came off, and the player in `slot`
			// moves (0x37); a player given a move back has had their undo.
			void undo(uint8_t count, uint8_t slot)
			{
				for (uint8_t i = 0; i < count && !stones.empty(); i++)
				{
					if (stones.back().color == my_color && my_stones > 0)
						my_stones--;

					stones.pop_back();
				}

				my_turn = slot == own_slot;

				if (my_turn)
					undo_used = true;

				new_turn();
				undo_asked = false;
				restart_timer();
			}

			void undo_refused()
			{
				undo_asked = false;
			}

			// The player in `slot` moves now, the last one's time having
			// run out (0x3f).
			void turn(uint8_t slot)
			{
				my_turn = slot == own_slot;
				new_turn();
				undo_asked = false;
				restart_timer();
			}

			void end(bool is_tie, uint8_t winner_slot) override
			{
				Game::end(is_tie, winner_slot);
				undo_asked = false;
			}

			// Counts down the turn by `ms`; true once, when the time runs
			// out on my turn and the server is to hear of it (0x6e4f68).
			bool tick(int32_t ms)
			{
				if (state != State::PLAYING)
					return false;

				timer = timer > ms ? timer - ms : 0;

				if (timer > 0 || timed_out || !my_turn)
					return false;

				timed_out = true;

				return true;
			}

			int32_t seconds_left() const
			{
				return (timer + 999) / 1000;
			}

			bool can_place(int32_t x, int32_t y) const
			{
				if (state != State::PLAYING || !my_turn)
					return false;

				for (const Stone &stone : stones)
					if (stone.x == x && stone.y == y)
						return false;

				return true;
			}

			// Taking a stone back needs one of mine on the board, once a
			// turn and once a game (0x6e8bc2).
			Undo can_ask_undo() const
			{
				if (state != State::PLAYING)
					return Undo::NO;

				if (undo_used)
					return Undo::USED;

				if (my_stones == 0 || undo_asked || undo_this_turn)
					return Undo::NO;

				return Undo::ASK;
			}

			void ask_undo()
			{
				undo_asked = true;
				undo_this_turn = true;
			}

			std::vector<Stone> stones;
			uint8_t my_color = 1;

		private:
			void restart_timer()
			{
				timer = TURN_MS;
				timed_out = false;
			}

			int32_t my_stones = 0;
			int32_t timer = TURN_MS;
			bool timed_out = false;
			bool undo_asked = false;
			bool undo_this_turn = false;
			bool undo_used = false;
		};

		// The match cards' grid for each board size (0x64dbac): "4 X 3",
		// "5 X 4" and "6 X 5", from the card count, the columns and the
		// first card's top-left corner.
		struct CardGrid
		{
			int32_t count;
			int32_t columns;
			int16_t left;
			int16_t top;
		};

		constexpr size_t CARD_SIZES = 3;

		inline CardGrid card_grid(uint8_t size)
		{
			switch (size)
			{
			case 1:
				return { 20, 5, 61, 72 };
			case 2:
				return { 30, 6, 33, 39 };
			default:
				return { 12, 4, 89, 106 };
			}
		}

		// A card is 49 by 62, one every 56 across and 67 down.
		constexpr int16_t CARD_WIDTH = 49;
		constexpr int16_t CARD_HEIGHT = 62;
		constexpr int16_t CARD_PITCH_X = 56;
		constexpr int16_t CARD_PITCH_Y = 67;

		inline std::pair<int16_t, int16_t> card_position(uint8_t size, int32_t index)
		{
			CardGrid grid = card_grid(size);

			return { static_cast<int16_t>(grid.left + CARD_PITCH_X * (index % grid.columns)),
				static_cast<int16_t>(grid.top + CARD_PITCH_Y * (index / grid.columns)) };
		}

		// The card under (x, y), or -1 (0x653a6b).
		inline int32_t card_at(uint8_t size, int16_t x, int16_t y)
		{
			int32_t count = card_grid(size).count;

			for (int32_t i = 0; i < count; i++)
			{
				auto at = card_position(size, i);

				if (x >= at.first && x < at.first + CARD_WIDTH && y >= at.second && y < at.second + CARD_HEIGHT)
					return i;
			}

			return -1;
		}

		// A match cards turn lasts 10 seconds; 11.6 after a miss, whose
		// first 1.6 the cards stay put (0x64e1c1). At the start each card
		// shows for a second, one every 200 ms (0x64efcc).
		constexpr int32_t CARD_TURN_MS = 10000;
		constexpr int32_t CARD_MISS_MS = 11600;
		constexpr int32_t CARD_START_MS = 11500;
		constexpr int32_t CARD_HIDE_MS = 900;
		constexpr int32_t CARD_REVEAL_MS = 200;
		constexpr int32_t CARD_REVEAL_STEPS = 5;

		// A match cards game as one player sees it (CMiniGameMatchDlg's
		// fields +0x730 to +0x7a0).
		class Cards : public Game
		{
		public:
			Cards(uint8_t own_slot, uint8_t size) : Game(own_slot), size(size < CARD_SIZES ? size : 0) {}

			// The game starts with `dealt` face down, after each has shown
			// for a moment; the player in `waits` moves second (0x3d).
			void start(uint8_t waits, std::vector<int32_t> dealt)
			{
				begin(waits);
				deck = std::move(dealt);
				shown.assign(deck.size(), false);
				pairs = { 0, 0 };
				first = -1;
				second = -1;
				picks = 0;
				hide_ms = 0;
				reveal_ms = 0;
				revealing = true;
				timer = static_cast<int32_t>(deck.size()) * CARD_REVEAL_MS + CARD_START_MS;
				timed_out = false;
			}

			// I may turn up `index` (0x64ece1).
			bool can_flip(int32_t index) const
			{
				return state == State::PLAYING && my_turn && picks < 2 && timer <= CARD_TURN_MS
					&& index >= 0 && index < static_cast<int32_t>(deck.size()) && deck[index] >= 0
					&& index != first;
			}

			// I turned up `index`: true for the turn's first card, which
			// shows at once. The second shows when the server says how it
			// went.
			bool flip(int32_t index)
			{
				picks++;

				if (picks == 1)
				{
					first = index;
					shown[index] = true;

					return true;
				}

				return false;
			}

			// The other player turned up their first card (0x44).
			void flipped_first(uint8_t index)
			{
				if (index >= deck.size())
					return;

				first = index;
				shown[index] = true;
			}

			// The turn's second card, `index`, after `first_index`: a pair
			// by the player in `slot`, who goes on, or a miss, after which
			// both turn back (0x44). True for a pair.
			bool flipped_second(uint8_t index, uint8_t first_index, uint8_t code)
			{
				if (index >= deck.size() || first_index >= deck.size())
					return false;

				bool matched = code >= 2;
				uint8_t slot = matched ? code - 2 : code;

				if (matched)
				{
					deck[index] = -1;
					deck[first_index] = -1;
					shown[index] = false;
					shown[first_index] = false;
					pairs[slot < 2 ? slot : 0]++;
					first = -1;
					second = -1;
					timer = CARD_TURN_MS;
				}
				else
				{
					shown[index] = true;
					shown[first_index] = true;
					first = first_index;
					second = index;
					hide_ms = CARD_HIDE_MS;
					my_turn = slot != own_slot;
					timer = CARD_MISS_MS;
				}

				timed_out = false;
				new_turn();
				picks = 0;

				return matched;
			}

			// The player in `slot` moves now, the last one's time having
			// run out; a card left up goes back (0x3f).
			void turn(uint8_t slot)
			{
				my_turn = slot == own_slot;
				hide_turned();
				new_turn();
				picks = 0;
				timer = CARD_TURN_MS;
				timed_out = false;
			}

			// Cards still up, a miss or the start's show, go back down.
			void end(bool is_tie, uint8_t winner_slot) override
			{
				Game::end(is_tie, winner_slot);
				hide_turned();
				shown.assign(deck.size(), false);
				revealing = false;
			}

			// Counts down by `ms`: the start's show, a miss turning back,
			// the turn's time. True once, when the time runs out on my turn
			// and the server is to hear of it (0x64efcc).
			bool tick(int32_t ms)
			{
				if (state != State::PLAYING)
					return false;

				if (revealing)
				{
					reveal_ms += ms;

					int32_t step = reveal_ms / CARD_REVEAL_MS;
					int32_t count = static_cast<int32_t>(deck.size());

					for (int32_t i = 0; i < count; i++)
						shown[i] = i <= step && step < i + CARD_REVEAL_STEPS;

					if (step >= count + CARD_REVEAL_STEPS)
						revealing = false;
				}

				if (hide_ms > 0)
				{
					hide_ms -= ms;

					if (hide_ms <= 0)
						hide_turned();
				}

				timer = timer > ms ? timer - ms : 0;

				if (timer > 0 || timed_out || !my_turn)
					return false;

				timed_out = true;

				return true;
			}

			// At most 10, even in the longer turn after a miss.
			int32_t seconds_left() const
			{
				return std::min<int32_t>((timer + 999) / 1000, CARD_TURN_MS / 1000);
			}

			bool face_up(int32_t index) const
			{
				return index >= 0 && index < static_cast<int32_t>(shown.size()) && shown[index];
			}

			bool removed(int32_t index) const
			{
				return index < 0 || index >= static_cast<int32_t>(deck.size()) || deck[index] < 0;
			}

			uint8_t size;
			std::vector<int32_t> deck;
			std::array<uint8_t, 2> pairs = { 0, 0 };

		private:
			void hide_turned()
			{
				for (int32_t index : { first, second })
					if (index >= 0 && index < static_cast<int32_t>(shown.size()))
						shown[index] = false;

				first = -1;
				second = -1;
				hide_ms = 0;
			}

			std::vector<bool> shown;
			int32_t first = -1;
			int32_t second = -1;
			int32_t picks = 0;
			int32_t hide_ms = 0;
			int32_t reveal_ms = 0;
			bool revealing = false;
			int32_t timer = CARD_TURN_MS;
			bool timed_out = false;
		};
	}
}
