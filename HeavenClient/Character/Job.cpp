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
#include "Job.h"

namespace ms
{
	Job::Job(uint16_t i)
	{
		change_job(i);
	}

	Job::Job()
	{
		change_job(0);
	}

	void Job::change_job(uint16_t i)
	{
		id = i;
		name = get_name(id);

		if (id == 0)
			level = Level::BEGINNER;
		else if (id % 100 == 0)
			level = Level::FIRST;
		else if (id % 10 == 0)
			level = Level::SECOND;
		else if (id % 10 == 1)
			level = Level::THIRD;
		else
			level = Level::FOURTH;
	}

	bool Job::is_sub_job(uint16_t subid) const
	{
		for (int32_t lvit = BEGINNER; lvit <= FOURTH; lvit++)
		{
			Level lv = static_cast<Level>(lvit);

			if (subid == get_subjob(lv))
				return true;
		}

		return false;
	}

	bool Job::can_use(int32_t skill_id) const
	{
		uint16_t required = static_cast<uint16_t>(skill_id / 10000);
		return is_sub_job(required);
	}

	uint16_t Job::get_id() const
	{
		return id;
	}

	uint16_t Job::get_subjob(Level lv) const
	{
		if (lv <= level)
		{
			switch (lv)
			{
				case Level::BEGINNER:
					return 0;
				case Level::FIRST:
					return (id / 100) * 100;
				case Level::SECOND:
					return (id / 10) * 10;
				case Level::THIRD:
					return (level == Level::FOURTH) ? id - 1 : id;
				case Level::FOURTH:
					return id;
			}
		}

		return 0;
	}

	const std::string &Job::get_name() const
	{
		return name;
	}

	Job::Level Job::get_level() const
	{
		return level;
	}

	// v83 FUN_004a77ef's table (report 243), spelling and spacing included: 82 of its 83 entries, all but 800's,
	// which is Korean text. The Cygnus Knights' tiers share the first's name. Any other job has none.
	std::string Job::get_name(uint16_t jid)
	{
		switch (jid)
		{
			case 0:
				return "Beginner";
			case 100:
				return "Swordman";
			case 110:
				return "Fighter";
			case 111:
				return "Crusader";
			case 112:
				return "Hero";
			case 120:
				return "Page";
			case 121:
				return "White Knight";
			case 122:
				return "Paladin";
			case 130:
				return "Spearman";
			case 131:
				return "Dragon Knight";
			case 132:
				return "Dark Knight";
			case 200:
				return "Magician";
			case 210:
				return "Wizard (Fire,Poison)";
			case 211:
				return "Mage(Fire, Poison)";
			case 212:
				return "Arch Mage(Fire,Poison)";
			case 220:
				return "Wizard(Ice,Lightning)";
			case 221:
				return "Mage(Ice,Lightning)";
			case 222:
				return "Arch Mage(Ice,Lightning)";
			case 230:
				return "Cleric";
			case 231:
				return "Priest";
			case 232:
				return "Bishop";
			case 300:
				return "Archer";
			case 310:
				return "Hunter";
			case 311:
				return "Ranger";
			case 312:
				return "Bowmaster";
			case 320:
				return "Crossbow man";
			case 321:
				return "Sniper";
			case 322:
				return "Marksman";
			case 400:
				return "Rogue";
			case 410:
				return "Assassin";
			case 411:
				return "Hermit";
			case 412:
				return "Night Lord";
			case 420:
				return "Bandit";
			case 421:
				return "Chief Bandit";
			case 422:
				return "Shadower";
			case 500:
				return "Pirate";
			case 510:
				return "Brawler";
			case 511:
				return "Marauder";
			case 512:
				return "Buccaneer";
			case 520:
				return "Gunslinger";
			case 521:
				return "Outlaw";
			case 522:
				return "Corsair";
			case 900:
				return "GM";
			case 910:
				return "SuperGM";
			case 920:
				return "MWLB";
			case 1000:
				return "Noblesse";
			case 1100:
			case 1110:
			case 1111:
			case 1112:
				return "Dawn Warrior";
			case 1200:
			case 1210:
			case 1211:
			case 1212:
				return "Blaze Wizard";
			case 1300:
			case 1310:
			case 1311:
			case 1312:
				return "Wind Archer";
			case 1400:
			case 1410:
			case 1411:
			case 1412:
				return "Night Walker";
			case 1500:
			case 1510:
			case 1511:
			case 1512:
				return "Thunder Breaker";
			case 2000:
				return "Legend";
			case 2100:
			case 2110:
			case 2111:
			case 2112:
				return "Aran";
			case 2001:
				return "Evan0";
			case 2200:
				return "Evan1";
			case 2210:
				return "Evan2";
			case 2211:
				return "Evan3";
			case 2212:
				return "Evan4";
			case 2213:
				return "Evan5";
			case 2214:
				return "Evan6";
			case 2215:
				return "Evan7";
			case 2216:
				return "Evan8";
			case 2217:
				return "Evan9";
			case 2218:
				return "Evan10";
			default:
				return "";
		}
	}
}