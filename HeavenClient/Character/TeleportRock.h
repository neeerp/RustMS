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

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// The maps kept for the teleport rocks (v83 CharacterData +0x563 and
	// +0x577): five for the regular rock and ten for the VIP one, an empty
	// slot holding EMPTY.
	class TeleportRock
	{
	public:
		static constexpr int32_t EMPTY = 999999999;

		// MAP_TRANSFER_RESULT's results that carry a list.
		static constexpr int8_t DROPPED = 2;
		static constexpr int8_t KEPT = 3;

		// StringPool 0xb8b and 0xb88.
		static constexpr const char *SAME_MAP = "It's the map you're currently on.";
		static constexpr const char *NOT_FOR_LIST = "This map is not available to enter for the list.";

		// 5040000 and 5040001 are regular rocks, 5041000 the VIP one
		// (FUN_00a0a63f).
		static bool is_vip(int32_t item_id)
		{
			return item_id / 1000 != 5040;
		}

		static size_t slots(bool vip)
		{
			return vip ? 10 : 5;
		}

		// Maple Island and the event maps can't be kept (FUN_0083a084).
		static bool may_keep(int32_t map_id)
		{
			return map_id / 100000000 != 0 && map_id / 1000000 % 100 != 9;
		}

		static int16_t used(const std::vector<int32_t> &maps)
		{
			return static_cast<int16_t>(std::count_if(maps.begin(), maps.end(), [](int32_t map_id)
			{
				return map_id != EMPTY;
			}));
		}

		// A typed name loses its blanks at both ends, as CString's TrimRight
		// and TrimLeft take them.
		static std::string trim(const std::string &name);

		// Why the current map can't be added to `maps`, or an empty string
		// (FUN_0083a084).
		static std::string keep_problem(const std::vector<int32_t> &maps, int32_t map_id);

		// Why a typed name, already trimmed, can't be the target, or an empty
		// string (FUN_0083a3c2).
		static std::string target_problem(const std::string &name, const std::string &own_name);

		// What a refusal in MAP_TRANSFER_RESULT says (FUN_00a25268), naming
		// `target` when they can't be found. Empty for anything else.
		static std::string refusal(int8_t result, const std::string &target);

		TeleportRock();

		// Replaces a list. Missing slots are empty, extra ones dropped.
		void set_maps(bool vip, std::vector<int32_t> maps);

		const std::vector<int32_t> &get_maps(bool vip) const;

		// The character last named in the window, for the notice when they
		// can't be found (v83 keeps it at ctx+0x36f4).
		void set_target(const std::string &name);

		const std::string &get_target() const;

	private:
		std::vector<int32_t> regular;
		std::vector<int32_t> vip;
		std::string target;
	};
}
