#pragma once

#include "../../Net/Handlers/Helpers/MobStatParser.h"

#include <algorithm>
#include <cstdint>
#include <map>
#include <vector>

namespace ms
{
	// The numbers every client shows over a mob that Poison, Venom or
	// Ninja Ambush hurts (CMob::Update, 0x6675a8): each second the status's
	// value, or the mob's info/fixedDamage if it has one, as a plain hit
	// shows. The server takes the HP; the number is only shown. A mob that
	// only some skills may hurt shows MISS for the others.
	class MobHurts
	{
	public:
		static constexpr int32_t INTERVAL = 1000;

		// Whether `skill` (0 for a basic attack) may hurt a mob with this
		// info/damagedBySelectedSkill list (0x68146c): any, if it's empty.
		// No skill hurts one with info/onlyNormalAttack.
		static bool hurt_by(const std::vector<int32_t> &damagedby, int32_t skill, bool onlynormal = false)
		{
			if (onlynormal && skill > 0)
				return false;

			return damagedby.empty() || std::find(damagedby.begin(), damagedby.end(), skill) != damagedby.end();
		}

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
									int32_t elapsed, const std::vector<int32_t> &damagedby = {},
									bool onlynormal = false)
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
					if (!hurt_by(damagedby, status->second.reason, onlynormal))
						due.push_back(0);
					else
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
