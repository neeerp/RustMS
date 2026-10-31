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
#include <cstdio>
#include <ctime>
#include <functional>
#include <optional>
#include <string>

namespace ms
{
	// The Scroll Lock screenshot key, as v83 handles VK 0x91 in FUN_009e40f1.
	namespace Screenshot
	{
		enum class Press
		{
			TAKE,
			// "You cannot take screenshots at this moment..." (0xb4b)
			REFUSE,
			IGNORE
		};

		// Presses less than a second apart count up while the status bar
		// is up (DAT_00bec208); the first three take a shot, the fourth is
		// refused with a notice and later ones do nothing. Without the status
		// bar every press takes one.
		class Throttle
		{
		public:
			Press press(int64_t now, bool in_game)
			{
				if (!last || now - *last > 999 || !in_game)
					count = 1;
				else
					count++;

				last = now;

				if (count < 4)
					return Press::TAKE;

				return count == 4 && in_game ? Press::REFUSE : Press::IGNORE;
			}

		private:
			std::optional<int64_t> last;
			int32_t count = 0;
		};

		// "%s\Maple%04d.jpg" (0x6b6), without the folder.
		inline std::string file_name(int32_t index)
		{
			char name[32];
			std::snprintf(name, sizeof(name), "Maple%04d.jpg", index);

			return name;
		}

		// "%s\%s_%04d%02d%02d_%02d%02d%02d.jpg" (FUN_00a26b05), without the
		// folder: a Lie Detector's shot, named for the player tested and the
		// time it was taken.
		inline std::string named_file(const std::string &name, const std::tm &at)
		{
			char stamp[32];
			std::snprintf(stamp, sizeof(stamp), "_%04d%02d%02d_%02d%02d%02d.jpg", at.tm_year + 1900, at.tm_mon + 1,
						  at.tm_mday, at.tm_hour, at.tm_min, at.tm_sec);

			return name + stamp;
		}

		// The index of the next shot (FUN_00745264): probe 0, 1, 3, 7, ...
		// until a file is missing, then bisect between the last taken probe
		// and it. Taken files are assumed to run without gaps.
		inline int32_t first_free(const std::function<bool(int32_t)>& taken)
		{
			int32_t probes = 1;

			while (taken(probes - 1))
				probes *= 2;

			int32_t low = probes / 2;
			int32_t high = probes - 1;

			while (low < high)
			{
				int32_t mid = (low + high) / 2;

				if (taken(mid))
					low = mid + 1;
				else
					high = mid;
			}

			return low;
		}
	}
}
