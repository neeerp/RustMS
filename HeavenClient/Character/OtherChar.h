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

#include <string>

#include <cstdint>

#include "Buff.h"
#include "Char.h"

#include "Look/CharLook.h"

#include "../Gameplay/MovePath.h"

#include <map>
#include <optional>
#include <utility>
#include <vector>

namespace ms
{
	// Other client's players.
	class OtherChar : public Char
	{
	public:
		OtherChar(int32_t charid, const CharLook &look, uint8_t level, int16_t job, const std::string &name,
				  int8_t stance, Point<int16_t> position);

		~OtherChar() override;

		// Update the character.
		int8_t update(const Physics &physics) override;

		// Queue a path for the character to play.
		void send_movement(const std::vector<Movement> &movements);

		// Update a skill level.
		void update_skill(int32_t skillid, uint8_t skilllevel);

		// Update the attack speed.
		void update_speed(uint8_t attackspeed);

		// Update the character look.
		void update_look(const LookEntry &look);

		// The ring blocks their look came with.
		const RingLooks &get_ring_looks() const;
		void set_ring_looks(const RingLooks &looks);

		// Show these buffs, each with its value, as well as those shown
		// already. Those the character spawned with (`init`) show without
		// animating.
		void show_buffs(const std::vector<std::pair<Buffstat::Id, int32_t>> &shown, bool init = false);

		// Stop showing these buffs.
		void hide_buffs(const std::vector<Buffstat::Id> &ended);

		// The value of a buff the character shows.
		std::optional<int32_t> get_buff(Buffstat::Id stat) const;

		// Return the character's attacking speed.
		int8_t get_integer_attackspeed() const override;

		// Return the character's level.
		uint16_t get_level() const override;

		int16_t get_job() const;

		// Return the character's level of a skill.
		int32_t get_skilllevel(int32_t skillid) const override;

		// Return the charge skill on the character's weapon, or 0.
		int32_t get_charge() const override;

		// Return the skill whose clone Shadow Partner draws, or 0 without the buff.
		int32_t get_shadow_partner() const override;

	private:
		// The effect of getting on or off `mount` (FUN_00936859).
		void show_mount_effect(int32_t mount, bool boarding);

		uint16_t level;
		int16_t job;
		move_path::Player path;

		std::unordered_map<int32_t, uint8_t> skilllevels;
		uint8_t attackspeed;
		std::map<Buffstat::Id, int32_t> buffs;
		RingLooks ring_looks;

		void show_diseases();
	};
}