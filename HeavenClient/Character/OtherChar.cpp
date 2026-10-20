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
#include "OtherChar.h"

#include "Diseases.h"
#include "MountRules.h"

#include "../Constants.h"
#include "../Gameplay/Combat/FlashJump.h"
#include "../Gameplay/Stage.h"

namespace ms
{
	OtherChar::OtherChar(int32_t id, const CharLook &lk, uint8_t lvl, int16_t jb, const std::string &nm, int8_t st,
						 Point<int16_t> pos) : Char(id, lk, nm),
											   path({ static_cast<double>(pos.x()), static_cast<double>(pos.y()), 0.0, 0.0, 0,
													  static_cast<uint8_t>(st) })
	{
		level = lvl;
		job = jb;
		set_position(pos);

		attackspeed = 6;
		attacking = false;
	}

	int8_t OtherChar::update(const Physics &physics)
	{
		path.step(Constants::TIMESTEP);

		const move_path::Player::State &now = path.get_state();

		if (!attacking)
			set_state(now.stance);

		phobj.hspeed = now.x - phobj.crnt_x();
		phobj.vspeed = now.y - phobj.crnt_y();
		phobj.move();

		// A Flash Jump in the path leaves its effect, by the player's job
		// (0x97feb8).
		if (path.take_arrived() & (1u << 6))
			if (const char *effect = flash_jump::remote_effect(job))
				Stage::get().show_flash_jump(effect, get_position(), facing_right);

		physics.get_fht().update_fh(phobj);

		bool aniend = Char::update(physics, get_stancespeed());

		if (aniend && attacking)
			attacking = false;

		return get_layer();
	}

	void OtherChar::send_movement(const std::vector<Movement> &newmoves)
	{
		path.receive(newmoves);
	}

	void OtherChar::update_skill(int32_t skillid, uint8_t skilllevel)
	{
		skilllevels[skillid] = skilllevel;
	}

	void OtherChar::update_speed(uint8_t as)
	{
		attackspeed = as;
	}

	void OtherChar::update_look(const LookEntry &newlook)
	{
		// The new look rides as the old did.
		int32_t riding = look.get_vehicle();
		look = newlook;
		look.set_vehicle(riding);
		look.set_tired(mount::tired(mount_level, mount_fatigue));

		set_state(path.get_state().stance);
	}

	void OtherChar::show_buffs(const std::vector<std::pair<Buffstat::Id, int32_t>> &shown, bool init)
	{
		for (auto [stat, value] : shown)
		{
			buffs[stat] = value;

			if (stat == Buffstat::Id::COMBO)
				show_combo(value - 1, init);

			// One spawned on a mount is on it at once (0x983cdd).
			if (stat == Buffstat::Id::MONSTER_RIDING)
				ride(value, init);
		}

		show_diseases();
	}

	void OtherChar::hide_buffs(const std::vector<Buffstat::Id> &ended)
	{
		for (Buffstat::Id stat : ended)
		{
			buffs.erase(stat);

			if (stat == Buffstat::Id::COMBO)
				show_combo(-1, false);

			if (stat == Buffstat::Id::MONSTER_RIDING)
				ride(0, false);
		}

		show_diseases();
	}

	// A disease another character shows has its reason for value.
	void OtherChar::show_diseases()
	{
		std::vector<int32_t> reasons;

		for (auto [stat, value] : buffs)
			if (disease::is_disease(stat))
				reasons.push_back(value);

		show_afflictions(reasons);
	}

	std::optional<int32_t> OtherChar::get_buff(Buffstat::Id stat) const
	{
		auto iter = buffs.find(stat);

		if (iter == buffs.end())
			return std::nullopt;

		return iter->second;
	}

	int8_t OtherChar::get_integer_attackspeed() const
	{
		return attackspeed;
	}

	uint16_t OtherChar::get_level() const
	{
		return level;
	}

	int32_t OtherChar::get_skilllevel(int32_t skillid) const
	{
		auto iter = skilllevels.find(skillid);

		if (iter == skilllevels.end())
			return 0;

		return iter->second;
	}
}