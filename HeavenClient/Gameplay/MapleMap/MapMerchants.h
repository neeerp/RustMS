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

#include "Layer.h"

#include "../Physics/Physics.h"
#include "../../Character/MiniRoomBalloon.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"

#include <map>
#include <optional>
#include <string>

namespace ms
{
	// The hired merchants standing on the current map (SPAWN_HIRED_MERCHANT,
	// 0x510e83): the employee, and over it the balloon of its room, made of
	// the permit's skin (0x48bbca).
	class MapMerchants
	{
	public:
		// A merchant's balloon under the cursor.
		struct Hit
		{
			int32_t owner;
			int32_t serial;
		};

		void spawn(int32_t owner, int32_t itemid, Point<int16_t> position, int16_t fh, std::string name,
				   const std::optional<MiniRoomBox> &box);

		// Show `box` over the merchant, or no balloon.
		void change(int32_t owner, const std::optional<MiniRoomBox> &box);

		void remove(int32_t owner);

		bool contains(int32_t owner) const;

		void update(const Physics &physics);

		// The skin behind the employee, the employee, and the skin in front
		// of it, on the layer of the merchant's foothold.
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// The signboards, with the mini room balloons (z 0xc0061440).
		void draw_balloons(double viewx, double viewy) const;

		void clear();

		// The balloon at `position` (in map coordinates), newest first.
		std::optional<Hit> at(Point<int16_t> position) const;

	private:
		struct Merchant
		{
			int32_t itemid;
			Point<int16_t> position;
			int16_t fh;
			std::optional<Layer::Id> layer;
			Animation employee;
			Text name;
			// The skin's pieces, shown only with a balloon.
			bool skinned = false;
			Texture backgrnd;
			Texture shop;
			Animation effect;
			MiniRoomBalloon balloon;
		};

		static void skin(Merchant &merchant, const std::optional<MiniRoomBox> &box);

		std::map<int32_t, Merchant> merchants;
	};
}
