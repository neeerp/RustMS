//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "MonsterBook.h"

namespace ms
{
	MonsterBook::MonsterBook()
	{
		cover = 0;
	}

	void MonsterBook::set_cover(int32_t cov)
	{
		cover = cov;
	}

	void MonsterBook::add_card(int16_t card, int8_t level)
	{
		set_card(FIRST_CARD + card, level);
	}

	void MonsterBook::set_card(int32_t card, int8_t level)
	{
		cards[card] = level;
	}

	int32_t MonsterBook::get_cover() const
	{
		return cover;
	}

	int8_t MonsterBook::get_level(int32_t card) const
	{
		auto iter = cards.find(card);

		return iter == cards.end() ? 0 : iter->second;
	}

	int16_t MonsterBook::count_cards(bool special) const
	{
		int16_t count = 0;

		for (auto &card : cards)
			if (card.second > 0 && is_special(card.first) == special)
				count++;

		return count;
	}

	int16_t MonsterBook::get_book_level() const
	{
		// Each level needs ten more cards than the last: 11 cards for
		// level 2, 31 for level 3, and so on. A 2009 book showed level 3 at
		// 35 cards.
		int16_t collected = count_cards(false) + count_cards(true);
		int16_t level = 0;
		int16_t next = 1;

		do
		{
			level++;
			next += level * 10;
		} while (collected >= next);

		return level;
	}

	bool MonsterBook::is_special(int32_t card)
	{
		return card / 1000 % 10 == 8;
	}
}
