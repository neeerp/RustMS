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

#include "../OutPacket.h"

#include "../Gameplay/Movement.h"

#include <algorithm>
#include <vector>

namespace ms
{
	// Base class for packets which update object movements with the server.
	class MovementPacket : public OutPacket
	{
	public:
		MovementPacket(OutPacket::Opcode opc) : OutPacket(opc) {}

	protected:
		// A path as v83 writes it (FUN_0068a563): where it starts, its
		// elements, the keys held (none here), and the rectangle around the
		// start and every element with a position.
		void writepath(Point<int16_t> start, const std::vector<Movement>& elements)
		{
			write_point(start);
			write_byte(static_cast<int8_t>(elements.size()));

			int16_t left = start.x();
			int16_t top = start.y();
			int16_t right = start.x();
			int16_t bottom = start.y();

			for (const Movement& element : elements)
			{
				writemovement(element);

				if (positioned(element.command))
				{
					left = std::min(left, element.xpos);
					top = std::min(top, element.ypos);
					right = std::max(right, element.xpos);
					bottom = std::max(bottom, element.ypos);
				}
			}

			write_byte(0);
			write_short(left);
			write_short(top);
			write_short(right);
			write_short(bottom);
		}

		static bool positioned(uint8_t command)
		{
			switch (command)
			{
				case 0:
				case 3:
				case 4:
				case 5:
				case 7:
				case 8:
				case 9:
				case 11:
				case 15:
				case 17:
					return true;
				default:
					return false;
			}
		}

		// One path element, laid out by its type as v83 writes it
		// (FUN_0068a563).
		void writemovement(const Movement& movement)
		{
			uint8_t command = movement.command;
			write_byte(command);

			switch (command)
			{
				case 0:
				case 5:
				case 15:
				case 17:
					write_short(movement.xpos);
					write_short(movement.ypos);
					write_short(movement.vx);
					write_short(movement.vy);
					write_short(movement.fh);

					// The foothold it fell from, which HC doesn't keep.
					if (command == 15)
						write_short(0);

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
					write_short(movement.vx);
					write_short(movement.vy);
					break;
				case 3:
				case 4:
				case 7:
				case 8:
				case 9:
				case 11:
					write_short(movement.xpos);
					write_short(movement.ypos);
					write_short(movement.fh);
					break;
				case 10:
					write_byte(movement.newstate);
					return;
				case 14:
					write_short(movement.vx);
					write_short(movement.vy);
					write_short(0);
					break;
				default:
					break;
			}

			write_byte(movement.newstate);
			write_short(movement.duration);
		}
	};
}