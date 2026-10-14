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
#pragma once

#include <cstdint>

#include <map>

namespace ms
{
	// Class that represents the monster card collection of an individual character
	class MonsterBook
	{
	public:
		// The first card (Snail); the set-field packet sends card ids as
		// offsets from it.
		static constexpr int32_t FIRST_CARD = 2380000;
		// A card's page fills at this many copies.
		static constexpr int8_t MAX_LEVEL = 5;

		MonsterBook();

		void set_cover(int32_t);

		// Add a card from the set-field packet, by its offset from FIRST_CARD.
		void add_card(int16_t, int8_t);

		void set_card(int32_t card, int8_t level);

		int32_t get_cover() const;

		// How many copies of the card were collected, 0 to 5.
		int8_t get_level(int32_t card) const;

		// How many different cards were collected, of special (boss) cards
		// or of the rest.
		int16_t count_cards(bool special) const;

		// The book's level, which grows with the different cards collected.
		int16_t get_book_level() const;

		// Whether a card is a special (boss) card, in the book's last tab.
		static bool is_special(int32_t card);

	private:
		int32_t cover;
		std::map<int32_t, int8_t> cards;
	};
}
