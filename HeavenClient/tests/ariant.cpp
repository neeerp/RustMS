#include "../Character/Ariant.h"

#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	struct Writer
	{
		std::vector<int8_t> bytes;

		template <typename T>
		Writer& put(T value)
		{
			int8_t raw[sizeof(T)];
			std::memcpy(raw, &value, sizeof(T));
			bytes.insert(bytes.end(), raw, raw + sizeof(T));
			return *this;
		}

		Writer& put_string(const std::string& text)
		{
			put<int16_t>(static_cast<int16_t>(text.size()));
			bytes.insert(bytes.end(), text.begin(), text.end());
			return *this;
		}

		ms::InPacket packet() const
		{
			return ms::InPacket(bytes.data(), bytes.size());
		}
	};

	std::string order(const ms::ariant::Board& board)
	{
		std::string names;

		for (const ms::ariant::Row& row : board.rows())
			names += row.name + "=" + std::to_string(row.score) + "#" + std::to_string(row.icon) + " ";

		return names;
	}
}

int main()
{
	try
	{
		using namespace ms;

		// ARIANT_ARENA_USER_SCORE: a count, then each name and its jewels.
		Writer two;
		two.put<uint8_t>(2).put_string("Alpha").put<int32_t>(3).put_string("Beta").put<int32_t>(-1);
		InPacket tworecv = two.packet();
		std::vector<ariant::Score> scores = ariant::read_scores(tworecv);
		require(!tworecv.available(), "the whole of the board's packet is read");
		require(scores.size() == 2 && scores[0].name == "Alpha" && scores[0].score == 3 &&
					scores[1].name == "Beta" && scores[1].score == -1,
				"each name and its score");

		Writer none;
		none.put<uint8_t>(0);
		InPacket nonerecv = none.packet();
		require(ariant::read_scores(nonerecv).empty() && !nonerecv.available(), "an empty board");

		// Rows come in the order they are first seen, and are drawn highest first.
		ariant::Board board;
		board.apply({ { "Alpha", 3 }, { "Beta", 5 }, { "Gamma", 3 } }, "Alpha", false);
		require(order(board) == "Beta=5#1 Alpha=3#0 Gamma=3#2 ", "highest first, ties as they came");

		// A score is the total, and the icon stays with its name.
		board.apply({ { "Gamma", 9 } }, "Alpha", false);
		require(order(board) == "Gamma=9#2 Beta=5#1 Alpha=3#0 ", "a new total moves the row up");

		// Scores past 9999 show 9999, and empty names are skipped.
		board.apply({ { "Beta", 12000 }, { "", 7 } }, "Alpha", false);
		require(order(board) == "Beta=9999#1 Gamma=9#2 Alpha=3#0 ", "capped, and no nameless row");

		// A negative score takes the name off; the next new row's icon is the count of rows.
		board.apply({ { "Gamma", -1 }, { "Delta", 4 } }, "Alpha", false);
		require(order(board) == "Beta=9999#1 Delta=4#2 Alpha=3#0 ", "off the board, and the icon repeats");

		// A GM keeps only their own row.
		ariant::Board gm;
		gm.apply({ { "Alpha", 3 }, { "Admin", 1 } }, "Admin", true);
		require(order(gm) == "Admin=1#0 ", "a GM sees only themselves");
		require(ariant::gm_job(900) && ariant::gm_job(910) && ariant::gm_job(800), "GM jobs");
		require(!ariant::gm_job(0) && !ariant::gm_job(2112) && !ariant::gm_job(1000) && !ariant::gm_job(700),
				"other jobs");

		// The board belongs to an arena, and goes with it.
		ariant::enter_field(ariant::FIELD_TYPE);
		ariant::board().apply({ { "Alpha", 1 } }, "Alpha", false);
		require(ariant::on_field() && ariant::board().rows().size() == 1, "an arena keeps its board");
		ariant::enter_field(0);
		require(!ariant::on_field() && ariant::board().rows().empty(), "leaving forgets it");

		require(ariant::score_text(7) == "   7" && ariant::score_text(9999) == "9999", "\"%4d\"");

		// Places on the scroll: ties share the lowest of their places.
		std::vector<ariant::Row> ranked = { { "A", 10, 0 }, { "B", 5, 1 }, { "C", 5, 2 }, { "D", 1, 3 } };
		require(ariant::places(ranked) == std::vector<int32_t>({ 1, 3, 3, 4 }), "ties share the lower place");
		std::vector<ariant::Row> level = { { "A", 0, 0 }, { "B", 0, 1 }, { "C", 0, 2 } };
		require(ariant::places(level) == std::vector<int32_t>({ 3, 3, 3 }), "all tied are last");
		require(ariant::places({}).empty(), "no rows, no places");
		require(ariant::place_text(1) == "1st" && ariant::place_text(2) == "2nd" &&
					ariant::place_text(3) == "3rd" && ariant::place_text(6) == "6th",
				"named places");
		require(ariant::place_text(7) == "7", "a number past the sixth");

		std::cout << "ariant: ok\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << "ariant: " << e.what() << '\n';
		return 1;
	}
}
