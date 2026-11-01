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
#include "Buff.h"

namespace ms
{
	namespace Buffstat
	{
		// The flag bits as v83 numbers them (1 << bit of a CFlag<128>). Bits
		// 82 and up are the two-state stats.
		Id by_bit(uint8_t bit)
		{
			switch (bit)
			{
			case 0: return WATK;
			case 1: return WDEF;
			case 2: return MATK;
			case 3: return MDEF;
			case 4: return ACC;
			case 5: return AVOID;
			case 6: return HANDS;
			case 7: return SPEED;
			case 8: return JUMP;
			case 9: return MAGIC_GUARD;
			case 10: return DARKSIGHT;
			case 11: return BOOSTER;
			case 12: return POWERGUARD;
			case 13: return HYPERBODYHP;
			case 14: return HYPERBODYMP;
			case 15: return INVINCIBLE;
			case 16: return SOULARROW;
			case 17: return STUN;
			case 18: return POISON;
			case 19: return SEAL;
			case 20: return DARKNESS;
			case 21: return COMBO;
			case 22: return WK_CHARGE;
			case 23: return DRAGONBLOOD;
			case 24: return HOLY_SYMBOL;
			case 25: return MESOUP;
			case 26: return SHADOWPARTNER;
			case 27: return PICKPOCKET;
			case 28: return MESOGUARD;
			case 29: return THAW;
			case 30: return WEAKEN;
			case 31: return CURSE;
			case 32: return SLOW;
			case 33: return MORPH;
			case 34: return RECOVERY;
			case 35: return MAPLE_WARRIOR;
			case 36: return STANCE;
			case 37: return SHARP_EYES;
			case 38: return MANA_REFLECTION;
			case 39: return SEDUCE;
			case 40: return SHADOW_CLAW;
			case 41: return INFINITY_;
			case 42: return HOLY_SHIELD;
			case 43: return HAMSTRING;
			case 44: return BLIND;
			case 45: return CONCENTRATE;
			case 46: return PUPPET;
			case 47: return ECHO_OF_HERO;
			case 49: return GHOST_MORPH;
			case 51: return CONFUSE;
			case 59: return BERSERK_FURY;
			case 60: return DIVINE_BODY;
			case 63: return FINALATTACK;
			case 64: return WIND_FINALATTACK;
			case 68: return ARAN_COMBO;
			case 69: return COMBO_DRAIN;
			case 70: return COMBO_BARRIER;
			case 71: return BODY_PRESSURE;
			case 72: return SMART_KNOCKBACK;
			case 73: return ITEM_AURA;
			case 74: return EXP_BUFF;
			case 75: return STOP_POTION;
			case 76: return STOP_MOTION;
			case 77: return FEAR;
			case 82: return ENERGY_CHARGE;
			case 83: return DASH_SPEED;
			case 84: return DASH_JUMP;
			case 85: return MONSTER_RIDING;
			case 86: return SPEED_INFUSION;
			case 87: return HOMING_BEACON;
			default: return NONE;
			}
		}
	}
}
