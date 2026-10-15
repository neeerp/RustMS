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

#include <cstddef>
#include <cstdint>
#include <vector>

#include "../../Graphics/Texture.h"
#include "../../Template/Interpolated.h"
#include "../../Template/Point.h"

namespace ms
{
	// A damage number or miss, laid out as in v83; see damage_layout.
	class DamageNumber
	{
	public:
		static const size_t NUM_TYPES = 4;

		enum Type
		{
			NORMAL,
			CRITICAL,
			TOPLAYER,
			HEAL
		};

		// A damage of 0 shows "Miss". The number is placed `lift` px above
		// its anchor.
		DamageNumber(Type type, int32_t damage, int16_t lift = 0);

		DamageNumber();

		void draw(double viewx, double viewy, float alpha) const;

		// Places the number: centred on the anchor's x, its digits' bottoms
		// 6 to 9 px below the anchor (less the lift), a miss's bottom on it.
		void set_anchor(Point<int16_t> anchor);

		bool update();

		static void init();

	private:
		struct Glyph
		{
			const Texture* texture;
			// Where the texture's origin is drawn, relative to the anchor.
			Point<int16_t> offset;
		};

		std::vector<Glyph> glyphs;
		Point<int16_t> anchor;
		int16_t lift = 0;
		int32_t elapsed = 0;
		Linear<float> rise;
		Linear<float> opacity;

		// Per type, the first digit's set ("1") and the others' ("0").
		static Texture digits[NUM_TYPES][2][10];
		static Texture misses[NUM_TYPES];
		static Texture critical_effect;
	};
}
