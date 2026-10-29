#include "Ariant.h"

#include <algorithm>
#include <cstdio>

namespace ms
{
	namespace ariant
	{
		namespace
		{
			bool arena = false;
		}

		std::vector<Score> read_scores(InPacket& recv)
		{
			std::vector<Score> scores;
			uint8_t count = static_cast<uint8_t>(recv.read_byte());

			for (uint8_t i = 0; i < count; i++)
			{
				std::string name = recv.read_string();
				int32_t score = recv.read_int();
				scores.push_back({ name, score });
			}

			return scores;
		}

		bool gm_job(uint16_t job)
		{
			int32_t group = (job % 1000) / 100;

			return group == 8 || group == 9;
		}

		void Board::apply(const std::vector<Score>& scores, const std::string& self, bool gm)
		{
			for (const Score& entry : scores)
			{
				if (entry.score < 0)
				{
					list.erase(std::remove_if(list.begin(), list.end(),
											  [&](const Row& row) { return row.name == entry.name; }),
							   list.end());
					continue;
				}

				if (entry.name.empty() || (gm && entry.name != self))
					continue;

				int32_t score = std::min(entry.score, MAX_SCORE);
				auto row = std::find_if(list.begin(), list.end(),
										[&](const Row& known) { return known.name == entry.name; });

				if (row != list.end())
					row->score = score;
				else
					list.push_back({ entry.name, score, list.size() });
			}

			std::stable_sort(list.begin(), list.end(),
							 [](const Row& a, const Row& b) { return a.score > b.score; });
		}

		const std::vector<Row>& Board::rows() const
		{
			return list;
		}

		void Board::clear()
		{
			list.clear();
		}

		void enter_field(int32_t fieldtype)
		{
			arena = fieldtype == FIELD_TYPE;
			board().clear();
		}

		bool on_field()
		{
			return arena;
		}

		Board& board()
		{
			static Board current;

			return current;
		}

		std::string score_text(int32_t score)
		{
			char text[16];
			std::snprintf(text, sizeof(text), "%4d", score);

			return text;
		}

		std::vector<int32_t> places(const std::vector<Row>& rows)
		{
			std::vector<int32_t> result(rows.size());
			int32_t place = 0;

			// From the last row up, a place changes only with the score.
			for (size_t i = rows.size(); i-- > 0;)
			{
				if (i + 1 == rows.size() || rows[i].score != rows[i + 1].score)
					place = static_cast<int32_t>(i + 1);

				result[i] = place;
			}

			return result;
		}

		std::string place_text(int32_t place)
		{
			static const char* const NAMES[] = { "1st", "2nd", "3rd", "4th", "5th", "6th" };

			if (place >= 1 && place <= 6)
				return NAMES[place - 1];

			return std::to_string(place);
		}
	}
}
