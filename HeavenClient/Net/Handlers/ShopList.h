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

#include "../InPacket.h"

#include <cstdint>
#include <cstring>
#include <vector>

namespace ms
{
	namespace shop_list
	{
		// Throwing stars and bullets come in sets that are recharged, not stacked
		inline bool is_rechargeable(int32_t itemid)
		{
			int32_t prefix = itemid / 10000;

			return prefix == 207 || prefix == 233;
		}

		struct Row
		{
			// The item's place in the server's list, which buying refers to
			int16_t index;
			int32_t itemid;
			int32_t price;
			int32_t pitch;
			int32_t time;
			// Mesos per star or bullet when recharging
			double unitprice;
			// How many can be bought at once: a stack, or one set
			int16_t buyable;

			// Rows without a price are only there to recharge
			bool for_sale() const
			{
				return price > 0;
			}
		};

		// The rows of OPEN_NPC_SHOP after the NPC id. Each item carries a
		// bundle size, or for stars and bullets a recharge rate (a double);
		// the item id says which.
		inline std::vector<Row> read(InPacket& recv)
		{
			std::vector<Row> rows;
			int16_t size = recv.read_short();

			for (int16_t i = 0; i < size; i++)
			{
				Row row = {};
				row.index = i;
				row.itemid = recv.read_int();
				row.price = recv.read_int();
				row.pitch = recv.read_int();
				row.time = recv.read_int();

				recv.skip(4);

				if (is_rechargeable(row.itemid))
				{
					int64_t bits = recv.read_long();
					std::memcpy(&row.unitprice, &bits, sizeof(bits));
				}
				else
				{
					recv.skip(2); // bundle size
				}

				row.buyable = recv.read_short();
				rows.push_back(row);
			}

			return rows;
		}
	}
}
