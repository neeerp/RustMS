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
#pragma once
#include "MobStatParser.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// The order DecodeTemporary reads the plain statuses in.
		constexpr uint8_t ORDER[] = {
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 18, 19, 16, 17,
			21, 22, 24, 25, 26, 28, 29, 30, 32, 33, 34
		};
	}

	std::vector<MobStatParser::Status> MobStatParser::parse(InPacket& recv, const BuffParser::Flags& flags)
	{
		std::vector<Status> statuses;

		for (uint8_t stat : ORDER)
		{
			if (!flags.has(stat))
				continue;

			int16_t value = recv.read_short();
			int32_t reason = recv.read_int();
			int16_t duration = recv.read_short();

			statuses.push_back({ stat, value, reason, duration });
		}

		if (flags.has(BURNED))
		{
			int32_t count = recv.read_int();

			recv.skip(12 * count);
		}

		auto find = [&statuses](uint8_t stat)
		{
			return std::find_if(statuses.begin(), statuses.end(),
								[stat](const Status &status) { return status.stat == stat; });
		};

		for (uint8_t stat : { PCOUNTER, MCOUNTER })
		{
			if (!flags.has(stat))
				continue;

			int32_t spread = recv.read_int();
			auto found = find(stat);

			if (found != statuses.end())
				found->spread = spread;
		}

		if (flags.has(PCOUNTER) || flags.has(MCOUNTER))
		{
			int32_t chance = recv.read_int();

			for (uint8_t stat : { PCOUNTER, MCOUNTER })
			{
				auto found = find(stat);

				if (found != statuses.end())
					found->chance = chance;
			}
		}

		if (flags.has(DISABLE))
		{
			int16_t value = static_cast<uint8_t>(recv.read_byte());
			recv.skip(1);

			statuses.push_back({ DISABLE, value, 0, 0 });
		}

		return statuses;
	}

	void MobStatParser::skip_reset(InPacket& recv, const BuffParser::Flags& flags)
	{
		if (flags.has(BURNED))
		{
			int32_t count = recv.read_int();

			recv.skip(4 * count);
		}
	}

	bool MobStatParser::stops(const BuffParser::Flags& flags)
	{
		return flags.has(STUN) || flags.has(FREEZE) || flags.has(DOOM) || flags.has(32);
	}
}
