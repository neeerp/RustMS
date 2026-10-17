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
#include "AttackHandlers.h"

#include "../Character/SkillId.h"
#include "../Gameplay/Combat/Prepare.h"
#include "../Gameplay/Stage.h"

#include <unordered_map>
#include <vector>

namespace ms
{
	AttackHandler::AttackHandler(Attack::Type t)
	{
		type = t;
	}

	void AttackHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		uint8_t count = recv.read_byte();

		recv.skip(1);

		AttackResult attack;
		attack.type = type;
		attack.attacker = cid;

		attack.level = recv.read_byte();
		attack.skill = (attack.level > 0) ? recv.read_int() : 0;

		attack.display = recv.read_byte();
		attack.toleft = recv.read_bool();
		attack.stance = recv.read_byte();
		attack.speed = recv.read_byte();

		recv.skip(1);

		attack.bullet = recv.read_int();

		attack.mobcount = (count >> 4) & 0xF;
		attack.hitcount = count & 0xF;

		for (uint8_t i = 0; i < attack.mobcount; i++)
		{
			int32_t oid = recv.read_int();

			recv.skip(1);

			uint8_t length = (attack.skill == SkillId::Id::MESO_EXPLOSION) ? recv.read_byte() : attack.hitcount;

			auto &lines = attack.damagelines.emplace_back(oid, std::vector<std::pair<int32_t, bool>>()).second;

			for (uint8_t j = 0; j < length; j++)
			{
				int32_t damage = recv.read_int();
				bool critical = false; // TODO: ?
				lines.push_back(std::make_pair(damage, critical));
			}
		}

		// A shot ends with where it went, and a held spell with its charge
		// (0x980645, 0x98066e).
		if (type == Attack::Type::RANGED)
			recv.skip(4);

		if (prepare::sends_charge(attack.skill, true))
			attack.charge = recv.read_int();

		Stage::get().get_combat().push_attack(attack);
	}

	void SkillEffectHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t skillid = recv.read_int();
		uint8_t level = recv.read_byte();
		uint16_t action_and_facing = static_cast<uint16_t>(recv.read_short());
		uint8_t speed = recv.read_byte();

		Stage::get().get_combat().show_prepare(cid, skillid, level, action_and_facing, speed);
	}

	void CancelSkillEffectHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t skillid = recv.read_int();

		Stage::get().get_combat().cancel_prepare(cid, skillid);
	}

	void ThrowGrenadeHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t x = recv.read_int();
		int32_t y = recv.read_int();
		int32_t charge = recv.read_int();
		int32_t skillid = recv.read_int();

		Point<int16_t> position(static_cast<int16_t>(x), static_cast<int16_t>(y));
		Stage::get().get_combat().show_throw(cid, position, charge, skillid);
	}
}
