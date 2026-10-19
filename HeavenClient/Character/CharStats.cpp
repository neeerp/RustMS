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
#include "CharStats.h"

#include "MobDamage.h"
#include "SecondaryStat.h"
#include "StatCaps.h"

#include "../Gameplay/Combat/Attack.h"
#include "../Gameplay/Combat/PhysicalDamage.h"

#include <algorithm>
#include <cmath>

namespace ms
{
	CharStats::CharStats(const StatsEntry &s) : name(s.name), petids(s.petids), exp(s.exp), mapid(s.mapid),
												portal(s.portal), rank(s.rank), jobrank(s.jobrank), basestats(s.stats),
												female(s.female)
	{
		job = basestats[MapleStat::Id::JOB];

		init_totalstats();
	}

	CharStats::CharStats()
	{}

	void CharStats::init_totalstats()
	{
		totalstats.clear();
		buffdeltas.clear();
		percentages.clear();

		totalstats[EquipStat::Id::HP] = get_stat(MapleStat::Id::MAXHP);
		totalstats[EquipStat::Id::MP] = get_stat(MapleStat::Id::MAXMP);
		totalstats[EquipStat::Id::STR] = get_stat(MapleStat::Id::STR);
		totalstats[EquipStat::Id::DEX] = get_stat(MapleStat::Id::DEX);
		totalstats[EquipStat::Id::INT] = get_stat(MapleStat::Id::INT);
		totalstats[EquipStat::Id::LUK] = get_stat(MapleStat::Id::LUK);
		totalstats[EquipStat::Id::SPEED] = 100;
		totalstats[EquipStat::Id::JUMP] = 100;

		maxdamage = 0;
		mindamage = 0;
		honor = 0;
		attackspeed = 0;
		projectilerange = 400;
		mastery = 0;
		// v83 has no base critical rate; only skills such as Critical Shot add one.
		critical = 0.0f;
		mincrit = 0.5f;
		maxcrit = 0.75f;
		bossdmg = 0.0f;
		ignoredef = 0.0f;
		stance = 0.0f;
		resiststatus = 0.0f;
		reducedamage = 0.0f;
	}

	void CharStats::close_totalstats()
	{
		// DEX and LUK add to accuracy and avoid by the job (SetFrom).
		int32_t id = job.get_id();
		int32_t dex = get_total(EquipStat::Id::DEX);
		int32_t luk = get_total(EquipStat::Id::LUK);
		totalstats[EquipStat::Id::ACC] += secondary_stat::base_accuracy(id, dex, luk);
		totalstats[EquipStat::Id::AVOID] += secondary_stat::base_avoid(id, dex, luk);

		for (auto iter : percentages)
		{
			EquipStat::Id stat = iter.first;
			int32_t total = totalstats[stat];
			total += static_cast<int32_t>(total * iter.second);
			set_total(stat, total);
		}

		// The window's range (FUN_008c2870): the least roll of a stab, and
		// the whole of a swing, between 1 and 199999.
		namespace pd = physical_damage;

		pd::Weapon weapon = pd::weapon(static_cast<int32_t>(weapontype) * 10000);
		pd::Stats stats{ get_total(EquipStat::Id::STR), dex, luk };
		int32_t pad = pd::attack(get_total(EquipStat::Id::WATK));
		auto shown = [](double damage) { return std::clamp(static_cast<int32_t>(damage), 1, 199999); };
		mindamage = shown(pd::lowest(pd::wielded(weapon, id, 0, false, stats, pd::least(mastery)), pad));
		maxdamage = shown(pd::highest(pd::wielded(weapon, id, 0, true, stats, 1.0), pad));
	}

	void CharStats::set_stat(MapleStat::Id stat, uint16_t value)
	{
		basestats[stat] = value;
	}

	void CharStats::set_total(EquipStat::Id stat, int32_t value)
	{
		auto iter = EQSTAT_CAPS.find(stat);

		if (iter != EQSTAT_CAPS.end())
		{
			int32_t cap_value = iter->second;

			if (value > cap_value)
				value = cap_value;
		}

		totalstats[stat] = value;
	}

	void CharStats::add_buff(EquipStat::Id stat, int32_t value)
	{
		int32_t current = get_total(stat);
		set_total(stat, current + value);
		buffdeltas[stat] += value;
	}

	void CharStats::add_value(EquipStat::Id stat, int32_t value)
	{
		int32_t current = get_total(stat);
		set_total(stat, current + value);
	}

	void CharStats::add_percent(EquipStat::Id stat, float percent)
	{
		percentages[stat] += percent;
	}

	void CharStats::set_weapontype(Weapon::Type w)
	{
		weapontype = w;
	}

	void CharStats::set_exp(int64_t e)
	{
		exp = e;
	}

	void CharStats::set_portal(uint8_t p)
	{
		portal = p;
	}

	void CharStats::set_mastery(int32_t m)
	{
		mastery = m;
	}

	void CharStats::set_reducedamage(float r)
	{
		reducedamage = r;
	}

	void CharStats::change_job(uint16_t id)
	{
		basestats[MapleStat::Id::JOB] = id;
		job.change_job(id);
	}

	int32_t CharStats::calculate_damage(const MobAttack &attack, double missroll, double roll, int32_t invincible) const
	{
		mob_damage::Defender player{
				get_stat(MapleStat::Id::LEVEL),
				get_stat(MapleStat::Id::JOB),
				get_total(EquipStat::Id::STR),
				get_total(EquipStat::Id::DEX),
				get_total(EquipStat::Id::INT),
				get_total(EquipStat::Id::LUK),
				get_total(EquipStat::Id::WDEF),
				get_total(EquipStat::Id::MDEF),
				get_total(EquipStat::Id::AVOID)
		};
		mob_damage::Attacker mob{ attack.level, attack.accuracy };

		if (attack.magic)
		{
			if (mob_damage::magic_misses(player, mob, missroll))
				return 0;

			return mob_damage::magic(player, attack.matk, roll, 0, attack.magicup);
		}

		if (missroll * 100.0 < mob_damage::physical_miss_chance(player, mob))
			return 0;

		return mob_damage::physical(player, mob, attack.watk, roll, invincible, attack.powerup);
	}

	bool CharStats::is_damage_buffed() const
	{
		return get_buffdelta(EquipStat::Id::WATK) > 0 || get_buffdelta(EquipStat::Id::MAGIC) > 0;
	}

	uint16_t CharStats::get_stat(MapleStat::Id stat) const
	{
		return basestats[stat];
	}

	int32_t CharStats::get_total(EquipStat::Id stat) const
	{
		return totalstats[stat];
	}

	int32_t CharStats::get_buffdelta(EquipStat::Id stat) const
	{
		return buffdeltas[stat];
	}

	Rectangle<int16_t> CharStats::get_range() const
	{
		return Rectangle<int16_t>(-projectilerange, -5, -50, 50);
	}

	void CharStats::set_mapid(int32_t id)
	{
		mapid = id;
	}

	int32_t CharStats::get_mapid() const
	{
		return mapid;
	}

	uint8_t CharStats::get_portal() const
	{
		return portal;
	}

	int64_t CharStats::get_exp() const
	{
		return exp;
	}

	const std::string &CharStats::get_name() const
	{
		return name;
	}

	const std::string &CharStats::get_jobname() const
	{
		return job.get_name();
	}

	Weapon::Type CharStats::get_weapontype() const
	{
		return weapontype;
	}

	int32_t CharStats::get_mastery() const
	{
		return mastery;
	}

	float CharStats::get_critical() const
	{
		return critical;
	}

	float CharStats::get_mincrit() const
	{
		return mincrit;
	}

	float CharStats::get_maxcrit() const
	{
		return maxcrit;
	}

	float CharStats::get_reducedamage() const
	{
		return reducedamage;
	}

	float CharStats::get_bossdmg() const
	{
		return bossdmg;
	}

	float CharStats::get_ignoredef() const
	{
		return ignoredef;
	}

	void CharStats::set_stance(float s)
	{
		stance = s;
	}

	float CharStats::get_stance() const
	{
		return stance;
	}

	float CharStats::get_resistance() const
	{
		return resiststatus;
	}

	int32_t CharStats::get_maxdamage() const
	{
		return maxdamage;
	}

	int32_t CharStats::get_mindamage() const
	{
		return mindamage;
	}

	uint16_t CharStats::get_honor() const
	{
		return honor;
	}

	void CharStats::set_attackspeed(int8_t as)
	{
		attackspeed = as;
	}

	int8_t CharStats::get_attackspeed() const
	{
		return attackspeed;
	}

	const Job &CharStats::get_job() const
	{
		return job;
	}

	bool CharStats::get_female() const
	{
		return female;
	}
}
