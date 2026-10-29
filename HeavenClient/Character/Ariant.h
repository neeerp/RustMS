#pragma once

#include "../Net/InPacket.h"

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The Ariant Coliseum's Battle Arena (the arena field, fieldType 13,
	// v83ref ariant-s4-hud): the server sends each player's Spirit Jewels,
	// and the client keeps them on a board at the top left of the screen.
	namespace ariant
	{
		constexpr int32_t FIELD_TYPE = 13;
		// The most a row shows (0x53e5e1).
		constexpr int32_t MAX_SCORE = 9999;

		// An entry of ARIANT_ARENA_USER_SCORE (0x129).
		struct Score
		{
			std::string name;
			int32_t score;
		};

		// The whole of ARIANT_ARENA_USER_SCORE: a count, then each name and
		// its score.
		std::vector<Score> read_scores(InPacket& recv);

		// Whether a job is a GM's: (job % 1000) / 100 is 8 or 9.
		bool gm_job(uint16_t job);

		// A row of the board. Its icon is the number of rows there were when
		// it came (characterIcon/%d), not its rank.
		struct Row
		{
			std::string name;
			int32_t score;
			size_t icon;
		};

		// The board, highest score first (0x53e5e1).
		class Board
		{
		public:
			// Applies the scores as the player `self` sees them:
			// - a negative score takes the rows of that name off;
			// - an empty name is skipped;
			// - a GM keeps only their own row;
			// - a known name gets its new score, capped at MAX_SCORE, and a new
			//   one goes at the end;
			// then the rows are sorted by score, ties staying in order.
			void apply(const std::vector<Score>& scores, const std::string& self, bool gm);
			const std::vector<Row>& rows() const;
			void clear();

		private:
			std::vector<Row> list;
		};

		// The field the server has moved the player to. Entering a field
		// forgets the board.
		void enter_field(int32_t fieldtype);
		// Whether that field is an arena.
		bool on_field();

		Board& board();

		// "%4d", as the scores are drawn.
		std::string score_text(int32_t score);

		// Each row's place on the results scroll (0x53e914). Rows with the
		// same score share the lowest of their places: 10, 5, 5, 1 are 1, 3,
		// 3, 4.
		std::vector<int32_t> places(const std::vector<Row>& rows);
		// "1st" to "6th", or the number past those (0x53e98e).
		std::string place_text(int32_t place);
	}
}
