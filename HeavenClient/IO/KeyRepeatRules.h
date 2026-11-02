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

namespace ms
{
	// v83 repeats a held key itself (GenerateAutoKeyDown, 0x59b2d2, run on
	// every pass of the main loop at 0x9f62c5): the last key to go down sends
	// its key-down again once the keyboard delay is up, then every repeat
	// interval while it stays down. Modifiers repeat too, so a held Ctrl keeps
	// attacking. These are the Windows defaults: clamp((delay + 1) * 250, 250,
	// 1000) and clamp(400 - 12 * speed, 33, 400) with SPI_GETKEYBOARDDELAY 1
	// and SPI_GETKEYBOARDSPEED 31.
	namespace key_repeat
	{
		constexpr int32_t NONE = -1;
		constexpr int64_t DELAY = 500;
		constexpr int64_t INTERVAL = 33;

		class Repeater
		{
		public:
			// A key going down becomes the one that repeats, after the delay.
			void press(int32_t keycode, int64_t now)
			{
				key = keycode;
				last = now;
				wait = DELAY;
			}

			// Letting go of another key leaves the repeat as it is; letting go
			// of this one ends it, and an earlier key still down doesn't take
			// over.
			void release(int32_t keycode)
			{
				if (keycode == key)
					stop();
			}

			void stop()
			{
				key = NONE;
			}

			// The key to send down again now, or NONE. The next one is timed
			// from now, as v83 times it.
			int32_t update(int64_t now)
			{
				if (key == NONE || now - last < wait)
					return NONE;

				last = now;
				wait = INTERVAL;

				return key;
			}

		private:
			int32_t key = NONE;
			int64_t last = 0;
			int64_t wait = DELAY;
		};
	}
}
