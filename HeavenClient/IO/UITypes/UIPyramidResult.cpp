#include "UIPyramidResult.h"

#include "../../Constants.h"
#include "../../Gameplay/Combat/Massacre.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where 0x5617c5 puts each layer, on the screen.
		const Point<int16_t> BACKGRD(207, 158);
		const Point<int16_t> RANK(233, 247);

		// In the board: the counts and their shares, in rows of kills, cool kills and misses, and the EXP.
		const int16_t COUNT_X = 218;
		const int16_t PERCENT_X = 265;
		const std::array<int16_t, 3> ROW_Y = { 63, 87, 111 };
		const Point<int16_t> EXP(258, 168);
		const int32_t EXP_SLOTS = 7;
		const int16_t EXP_ADVANCE = 15;

		const std::array<const char *, 5> RANKS = { "s", "a", "b", "c", "d" };

		// 0x1388: the board's lifetime.
		const int16_t SHOWN_FOR = 5000;
	}

	UIPyramidResult::UIPyramidResult(int8_t rank_id, int32_t gained)
		: UIElement(Point<int16_t>(), Point<int16_t>()), exp(gained), shown(0)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MonsterKilling"]["Result"];

		backgrd = src["backgrd"];
		backgrd2 = src["backgrd2"];

		for (size_t i = 0; i < number.size(); i++)
		{
			number[i] = src["number"][std::to_string(i)];
			number2[i] = src["number2"][std::to_string(i)];
		}

		if (rank_id >= 0 && rank_id < static_cast<int8_t>(RANKS.size()))
			rank = src["Rank"][RANKS[rank_id]];

		// The counts the server kept for the run, as they stand.
		const massacre::Session &state = massacre::session();
		counts = { state.hit, state.cool, state.miss };
	}

	void UIPyramidResult::draw_number(const std::array<Texture, 10> &font, Point<int16_t> pos, int32_t slots,
									  int16_t advance, int32_t value) const
	{
		for (auto [slot, digit] : massacre::digits(value, slots))
			font[digit].draw(pos + Point<int16_t>(static_cast<int16_t>(advance * slot), 0));
	}

	void UIPyramidResult::draw(float alpha) const
	{
		backgrd.draw(position + BACKGRD);

		for (size_t i = 0; i < counts.size(); i++)
		{
			Point<int16_t> row = position + BACKGRD + Point<int16_t>(0, ROW_Y[i]);
			draw_number(number, row + Point<int16_t>(COUNT_X, 0), massacre::COUNT_SLOTS, massacre::COUNT_ADVANCE,
						counts[i]);
			draw_number(number, row + Point<int16_t>(PERCENT_X, 0), massacre::COUNT_SLOTS, massacre::COUNT_ADVANCE,
						massacre::percent(counts[i], counts[0], counts[1], counts[2]));
		}

		draw_number(number2, position + BACKGRD + EXP, EXP_SLOTS, EXP_ADVANCE, exp);

		backgrd2.draw(position + RANK, alpha);
		rank.draw(position + RANK);
	}

	void UIPyramidResult::update()
	{
		UIElement::update();

		backgrd2.update();
		shown += Constants::TIMESTEP;

		if (shown >= SHOWN_FOR)
			deactivate();
	}

	bool UIPyramidResult::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIPyramidResult::get_type() const
	{
		return TYPE;
	}
}
