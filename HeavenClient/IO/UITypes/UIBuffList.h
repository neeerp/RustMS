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

#include "../UIElement.h"
#include "../Constants.h"

#include "../Components/BuffRow.h"
#include "../Graphics/Texture.h"

namespace ms
{
	// v83's buff icons in the top right (FUN_007b2bb0): 32x32 cells ending
	// 3 px from the right edge at y 23. Each has IconBase/0 under the
	// buff's icon, centred, with the timer's CoolTime frame on top. Hovering
	// over one shows its skill's tooltip without levels, or its item's.
	class UIBuffList : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::BUFFLIST;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIBuffList();

		void draw(float inter) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		Cursor::State send_cursor(bool pressed, Point<int16_t> position) override;
		bool is_in_range(Point<int16_t> cursorpos) const override;
		void remove_cursor() override;
		void rightclick(Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		// Item buffs have negative ids.
		void add_buff(int32_t buffid, int32_t stat, int32_t duration);
		void cancel_buff(int32_t stat);
		// The Battleship's HP: 0 takes its icon away.
		void set_gauge(int32_t buffid, int32_t value);
		// The family entitlement in use, `left` ms from its end, in place
		// of any other: Family/RightIcon/<icon>. 0 takes it away.
		void set_family(uint8_t icon, int32_t left);

	private:
		static constexpr int16_t CELL = 32;
		// The family entitlement's id and stat in the row, which no skill
		// or item has.
		static constexpr int32_t FAMILY = INT32_MIN;

		void show_family() const;

		// The buff under the cursor (FUN_007b3055), if any.
		const BuffRow<Texture>::Entry* entry_at(Point<int16_t> cursorpos) const;

		BuffRow<Texture> row;
		Texture base;
		Texture shadows[BuffTimer::SHADOWS];
	};
}
