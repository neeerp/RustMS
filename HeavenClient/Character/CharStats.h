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

#include "Job.h"

#include "../Net/Login.h"
#include "../Template/Rectangle.h"

namespace ms
{
	struct MobAttack;

	class CharStats
	{
	public:
		CharStats(const StatsEntry &entry);

		CharStats();

		void init_totalstats();

		void set_stat(MapleStat::Id stat, uint16_t value);

		void set_total(EquipStat::Id stat, int32_t value);

		void add_buff(EquipStat::Id stat, int32_t value);

		void add_value(EquipStat::Id stat, int32_t value);

		void add_percent(EquipStat::Id stat, float percent);
		
		void close_totalstats();

		void set_weapontype(Weapon::Type weapontype);

		void set_exp(int64_t exp);

		// EXP from a Writ of Solomon, taken by double-clicking the EXP gauge.
		void set_gachaexp(int32_t exp);

		void set_portal(uint8_t portal);

		// The mastery the window's damage range counts.
		void set_mastery(int32_t mastery);

		void set_reducedamage(float reducedamage);

		void change_job(uint16_t id);

		// What a mob's touch or attack takes from the player, or 0 when it
		// misses. `missroll` and `roll` are uniform in [0, 1).
		// A mob's hit, less what Invincible (`invincible` percent) takes off
		// a physical one.
		int32_t calculate_damage(const MobAttack &attack, double missroll, double roll, int32_t invincible) const;

		bool is_damage_buffed() const;

		uint16_t get_stat(MapleStat::Id stat) const;

		int32_t get_total(EquipStat::Id stat) const;

		int32_t get_buffdelta(EquipStat::Id stat) const;
		
		Rectangle<int16_t> get_range() const;

		void set_mapid(int32_t id);

		int32_t get_mapid() const;

		uint8_t get_portal() const;

		int64_t get_exp() const;

		int32_t get_gachaexp() const;

		const std::string &get_name() const;

		const std::string &get_jobname() const;

		Weapon::Type get_weapontype() const;

		int32_t get_mastery() const;

		float get_critical() const;

		float get_mincrit() const;

		float get_maxcrit() const;

		float get_reducedamage() const;

		float get_bossdmg() const;

		float get_ignoredef() const;

		void set_stance(float stance);

		float get_stance() const;

		float get_resistance() const;

		int32_t get_maxdamage() const;

		int32_t get_mindamage() const;

		uint16_t get_honor() const;

		void set_attackspeed(int8_t speed);

		int8_t get_attackspeed() const;

		const Job &get_job() const;

		bool get_female() const;

	private:
		std::string name;
		std::vector<int64_t> petids;
		Job job;
		int64_t exp;
		int32_t gachaexp = 0;
		int32_t mapid;
		uint8_t portal;
		std::pair<int32_t, int8_t> rank;
		std::pair<int32_t, int8_t> jobrank;
		EnumMap<MapleStat::Id, uint16_t> basestats;
		EnumMap<EquipStat::Id, int32_t> totalstats;
		EnumMap<EquipStat::Id, int32_t> buffdeltas;
		EnumMap<EquipStat::Id, float> percentages;
		int32_t maxdamage;
		int32_t mindamage;
		uint16_t honor;
		int8_t attackspeed;
		int16_t projectilerange;
		Weapon::Type weapontype;
		int32_t mastery;
		float critical;
		float mincrit;
		float maxcrit;
		float bossdmg;
		float ignoredef;
		float stance;
		float resiststatus;
		float reducedamage;
		bool female;
	};
}
