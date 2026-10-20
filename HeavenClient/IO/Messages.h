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

#include "../Character/Inventory/Weapon.h"
#include "../Gameplay/Combat/SpecialMove.h"

#include <string>

namespace ms
{
	namespace Messages
	{
		enum Type
		{
			NONE,
			// Cannot use a skill
					SKILL_WEAPONTYPE,
			SKILL_EQUIPPED,
			SKILL_HPCOST,
			SKILL_MPCOST,
			SKILL_MESOCOST,
			// Names the item in place of %s
			SKILL_ITEMCOST,
			SKILL_NOARROWS,
			SKILL_NOBULLETS,
			SKILL_NOSTARS,
			// Shadow Stars finds no stack of stars
			SKILL_SHADOWSTARS,
			SKILL_COOLDOWN,
			SKILL_ONEHANDED,
			// StringPool 0xdd1
			SKILL_ENERGY,
			// The map forbids the skill (fieldLimit)
			SKILL_FIELD,
			// A Mystic Door may not open by a portal
			DOOR_PORTAL,
			// Attacking without a weapon
			ATTACK_NOWEAPON,
			// The map forbids the item (fieldLimit)
			ITEM_FIELD,
			DROP_FIELD,
			// The item's info/maxLevel is below the player's level
			ITEM_LEVEL_HIGH,
			// A Writ of Solomon while gacha EXP is left
			SOLOMON_EXP_LEFT,
			// Not while riding a tamed monster, or another mount (FUN_0095f9d9)
			RIDING_MONSTER,
			RIDING_VEHICLE,
			// Getting on a tamed monster without one or a saddle, or one that can't be ridden (0x968b9d)
			RIDE_NO_SADDLE,
			RIDE_REFUSED,
			// A chair in a field of type 61 (0x10ad)
			CHAIR_FIELD,
			// Not while an item's morph lasts, one that can't be cancelled or else one that can
			// (0xdcf, 0xdd0)
			MORPHED_FIXED,
			MORPHED,
			// Right-clicking such a morph item's icon (0x10fc)
			MORPH_FIXED,
			LENGTH
		};

		extern const EnumMap<Type, const char *> messages;

		// Whether the line says what a skill costs. v83 says these at most
		// every two seconds between them.
		bool is_cost(Type type);
	};

	class InChatMessage
	{
	public:
		InChatMessage(Messages::Type type, std::string item = "");

		void drop() const;

	private:
		Messages::Type type;
		std::string item;
	};

	class ForbidSkillMessage : public InChatMessage
	{
	public:
		// item: the item the skill takes, for FBR_ITEMCOST
		ForbidSkillMessage(SpecialMove::ForbidReason reason, Weapon::Type weapon, int32_t item = 0);

	private:
		Messages::Type message_by_reason(SpecialMove::ForbidReason reason, Weapon::Type weapon);

		Messages::Type message_by_weapon(Weapon::Type weapon);
	};
}