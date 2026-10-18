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
#include "MovementParser.h"

namespace ms
{
	std::vector<Movement> MovementParser::parse_movements(InPacket& recv, Point<int16_t> start)
	{
		std::vector<Movement> movements;
		uint8_t length = recv.read_byte();
		int16_t x = start.x();
		int16_t y = start.y();
		uint8_t stance = 0;

		for (uint8_t i = 0; i < length; ++i)
		{
			Movement fragment;
			fragment.command = recv.read_byte();

			switch (fragment.command)
			{
			case 0:
			case 5:
			case 15:
			case 17:
				// A move to a point, at a speed, onto a foothold; a jump down
				// also says the foothold it fell from.
				fragment.type = fragment.command == 15 ? Movement::JUMPDOWN : Movement::ABSOLUTE;
				x = recv.read_short();
				y = recv.read_short();
				fragment.vx = recv.read_short();
				fragment.vy = recv.read_short();
				fragment.fh = recv.read_short();

				if (fragment.command == 15)
					recv.skip(2);

				break;
			case 1:
			case 2:
			case 6:
			case 12:
			case 13:
			case 16:
			case 18:
			case 19:
			case 20:
			case 22:
				// A speed from where the last element left off.
				fragment.type = Movement::RELATIVE;
				fragment.vx = recv.read_short();
				fragment.vy = recv.read_short();
				break;
			case 3:
			case 4:
			case 7:
			case 8:
			case 9:
			case 11:
				// A point and a foothold with no speed: a teleport (4), a seat
				// (11) and the like.
				fragment.type = fragment.command == 11 ? Movement::CHAIR : Movement::ABSOLUTE;
				x = recv.read_short();
				y = recv.read_short();
				fragment.fh = recv.read_short();
				break;
			case 10:
				// An equip or stat change: one byte, with no stance or time.
				fragment.type = Movement::NONE;
				recv.skip(1);
				break;
			case 14:
				fragment.type = Movement::RELATIVE;
				fragment.vx = recv.read_short();
				fragment.vy = recv.read_short();
				recv.skip(2);
				break;
			default:
				fragment.type = Movement::NONE;
				break;
			}

			fragment.xpos = x;
			fragment.ypos = y;

			if (fragment.command != 10)
			{
				stance = recv.read_byte();
				fragment.duration = recv.read_short();
			}

			fragment.newstate = stance;
			movements.push_back(fragment);
		}

		return movements;
	}
}
