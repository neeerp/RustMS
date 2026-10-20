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
#include "../Gameplay/Combat/Dash.h"
#include "../Gameplay/Combat/FlashJump.h"
#include "../Gameplay/Combat/RecoilShot.h"
#include "../Gameplay/Combat/Wings.h"
#include "../Gameplay/Stage.h"
#include "../IO/UITypes/EnergyBarLayout.h"

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

	OtherChar::~OtherChar()
	{
		glide(false, false);
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
		uint32_t arrived = path.take_arrived();

		if (arrived & (1u << 6))
			if (const char *effect = flash_jump::remote_effect(job))
				Stage::get().show_flash_jump(effect, get_position(), facing_right);

		// So does a Recoil Shot (0x980104).
		if (arrived & (1u << recoil_shot::MOVE))
			Stage::get().get_combat().show_recoil_shot(*this);

		// Wings' glide shows while the path glides down (0x9b17ec).
		glide(wings::gliding(path.command() == wings::GLIDE, now.vy), now.fh > 0);

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
		// The new look rides, and is morphed, as the old was.
		int32_t riding = look.get_vehicle();
		int32_t morph = look.get_morph();
		look = newlook;
		look.set_vehicle(riding);
		look.set_morph(morph);
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

			if (stat == Buffstat::Id::MORPH)
				transform(value, init, Stage::get().get_mapinfo().is_underwater());

			// The glow is Energy Charge's for the character's line (0x94b44b).
			if (stat == Buffstat::Id::ENERGY_CHARGE)
				show_energy(job / 1000 == 1 ? 15100004 : 5110001, value >= energy_bar::FULL, init);

			if (stat == Buffstat::Id::DASH_SPEED)
				show_dash(dash::skill(job));
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

			if (stat == Buffstat::Id::MORPH)
				transform(0, false);

			if (stat == Buffstat::Id::ENERGY_CHARGE)
				show_energy(0, false, false);

			if (stat == Buffstat::Id::DASH_SPEED)
				show_dash(0);
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

	int16_t OtherChar::get_job() const
	{
		return job;
	}

	int32_t OtherChar::get_skilllevel(int32_t skillid) const
	{
		auto iter = skilllevels.find(skillid);

		if (iter == skilllevels.end())
			return 0;

		return iter->second;
	}

	int32_t OtherChar::get_charge() const
	{
		return get_buff(Buffstat::Id::WK_CHARGE).value_or(0);
	}
}