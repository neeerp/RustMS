#pragma once

#include "../../Net/Handlers/Helpers/MobStatParser.h"

#include <cstdint>
#include <map>
#include <vector>

namespace ms
{
	// The numbers every client shows over a mob that Poison, Venom or
	// Ninja Ambush hurts (CMob::Update, 0x6675a8): each second the status's
	// value, or the mob's info/fixedDamage if it has one, as a plain hit
	// shows. The server takes the HP; the number is only shown. A mob that
	// only some skills may hurt shows MISS for the others (0x68146c); that
	// isn't done here.
	class MobHurts
	{
	public:
		static constexpr int32_t INTERVAL = 1000;

		static bool hurts(uint8_t stat)
		{
			return stat == MobStatParser::POISON || stat == MobStatParser::VENOM || stat == MobStatParser::AMBUSH;
		}

		// A status was put on: its first number comes a second on.
		void start(uint8_t stat)
		{
			if (hurts(stat))
				clocks[stat] = 0;
		}

		// Move `elapsed` ms on, returning the numbers due, in the client's
		// order: Poison, Venom, Ambush.
		std::vector<int32_t> update(const std::map<uint8_t, MobStatParser::Status> &statuses, int32_t fixeddamage,
									int32_t elapsed)
		{
			std::vector<int32_t> due;

			for (uint8_t stat : { MobStatParser::POISON, MobStatParser::VENOM, MobStatParser::AMBUSH })
			{
				auto status = statuses.find(stat);

				if (status == statuses.end() || status->second.value == 0)
				{
					clocks.erase(stat);
					continue;
				}

				int32_t &clock = clocks[stat];
				clock += elapsed;

				if (clock >= INTERVAL)
				{
					clock = 0;
					due.push_back(fixeddamage > 0 ? fixeddamage : status->second.value);
				}
			}

			return due;
		}

	private:
		// The ms since each status's last number.
		std::map<uint8_t, int32_t> clocks;
	};
}
