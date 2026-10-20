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
#include "Messages.h"

#include "UI.h"

#include "UITypes/UIChatBar.h"

#include "../Data/ItemData.h"

namespace ms
{
	const EnumMap<Messages::Type, const char *> Messages::messages =
			{
					"",
					"You cannot use this skill with the weapon you're equipped with.",
					"This skill cannot be used with the equipped item.",
					"You don't have enough HP to use this skill.",
					"You don't have enough MP to use this skill.",
					"You don't have enough mesos to use this skill.",
					"You don't have enough %s to use this skill.",
					"You have used up all of your arrows.",
					"You do not have enough bullets to use this skill.",
					"You have used up all of your throwing-stars.",
					"You do not have enough throwing stars to use this skill.",
					"You may not use the skill yet.",
					"You may only use this skill with a one-handed weapon equipped.",
					"You cannot use that skill on this map.",
					"You cannot use the Mystic Door Skill near the portal.",
					"Since you are not using a weapon, you are unable to attack.",
					"This item cannot be used here.",
					"You cannot drop the item here.",
					"Your level is too high to use the selected item.",
					"You'll need to use all your remaining EXP in order to use a new Writ of Solomon.",
					"Not available while riding on the tamed monster.",
					"Not available during the ride.",
					"This skill requires a tamed monster and a saddle.",
					"You cannot ride on this tamed monster.",
					"It is not appropriate to use it here."
			};

	bool Messages::is_cost(Type type)
	{
		return type == SKILL_HPCOST || type == SKILL_MPCOST || type == SKILL_MESOCOST || type == SKILL_ITEMCOST;
	}

	InChatMessage::InChatMessage(Messages::Type t, std::string i) : type(t), item(std::move(i))
	{}

	void InChatMessage::drop() const
	{
		if (type == Messages::Type::NONE)
			return;

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->display_message(type, UIChatBar::SKILL, item);
	}


	ForbidSkillMessage::ForbidSkillMessage(SpecialMove::ForbidReason reason, Weapon::Type weapon, int32_t item)
			: InChatMessage(message_by_reason(reason, weapon), item ? ItemData::get(item).get_name() : "")
	{}

	Messages::Type ForbidSkillMessage::message_by_reason(SpecialMove::ForbidReason reason, Weapon::Type weapon)
	{
		switch (reason)
		{
			case SpecialMove::FBR_WEAPONTYPE:
				return Messages::SKILL_WEAPONTYPE;
			case SpecialMove::FBR_EQUIPPED:
				return Messages::SKILL_EQUIPPED;
			case SpecialMove::FBR_HPCOST:
				return Messages::SKILL_HPCOST;
			case SpecialMove::FBR_MPCOST:
				return Messages::SKILL_MPCOST;
			case SpecialMove::FBR_MESOCOST:
				return Messages::SKILL_MESOCOST;
			case SpecialMove::FBR_ITEMCOST:
				return Messages::SKILL_ITEMCOST;
			case SpecialMove::FBR_COOLDOWN:
				return Messages::SKILL_COOLDOWN;
			case SpecialMove::FBR_ONEHANDED:
				return Messages::SKILL_ONEHANDED;
			case SpecialMove::FBR_BULLETCOST:
				return message_by_weapon(weapon);
			default:
				return Messages::NONE;
		}
	}

	Messages::Type ForbidSkillMessage::message_by_weapon(Weapon::Type weapon)
	{
		switch (weapon)
		{
			case Weapon::BOW:
			case Weapon::CROSSBOW:
				return Messages::SKILL_NOARROWS;
			case Weapon::CLAW:
				return Messages::SKILL_NOSTARS;
			default:
				return Messages::SKILL_NOBULLETS;
		}
	}
}