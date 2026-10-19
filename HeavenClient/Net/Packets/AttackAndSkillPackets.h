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

#include <cstdint>
#include <optional>
#include <vector>

#include "../OutPacket.h"

#include "../Gameplay/Combat/Attack.h"
#include "../Gameplay/Combat/Prepare.h"
#include "../Gameplay/Combat/SkillUse.h"

namespace ms
{
	// Notifies the server of an attack. The opcode is determined by the attack type.
	// Attack::CLOSE = CLOSE_ATTACK(44)
	// Attack::RANGED = RANGED_ATTACK(45)
	// Attack::MAGIC = MAGIC_ATTACK(46)
	class AttackPacket : public OutPacket
	{
	public:
		// A close attack ends with where the user stands, and Poison Bomb's
		// with where its grenade went off too (0x952b58).
		AttackPacket(const AttackResult& attack, Point<int16_t> position, std::optional<Point<int16_t>> grenade = std::nullopt) : OutPacket(opcodefor(attack.type))
		{
			skip(1);

			write_byte((attack.mobcount << 4) | attack.hitcount);
			write_int(attack.skill);

			skip(8);

			// How long a held skill was charged, sent even when it is 0
			// (0x95292e, 0x955210, 0x956e66).
			if (prepare::sends_charge(attack.skill, attack.type == Attack::Type::MAGIC))
				write_int(attack.charge);

			write_byte(attack.display);
			// The action, with the facing in the top bit (0x952974).
			write_short(static_cast<int16_t>((attack.stance & 0x7FFF) | (attack.toleft ? 0x8000 : 0)));

			skip(1);

			write_byte(attack.speed);

			if (attack.type == Attack::Type::RANGED)
			{
				write_int(0);
				write_short(attack.bulletslot);
				write_short(0);
				write_byte(0);
			}
			else
			{
				skip(4);
			}

			for (auto& damagetomob : attack.damagelines)
			{
				write_int(damagetomob.first);

				skip(14);

				for (auto& singledamage : damagetomob.second)
				{
					write_int(singledamage.first);
					// TODO: Add critical here
				}

				if (attack.skill != 5221004)
					skip(4);
			}

			if (attack.type == Attack::Type::CLOSE)
			{
				write_short(position.x());
				write_short(position.y());

				if (attack.skill == prepare::POISON_BOMB && grenade)
				{
					write_short(grenade->x());
					write_short(grenade->y());
				}
			}
		}

	private:
		static OutPacket::Opcode opcodefor(Attack::Type type)
		{
			switch (type)
			{
			case Attack::Type::CLOSE:
				return OutPacket::Opcode::CLOSE_ATTACK;
			case Attack::Type::RANGED:
				return OutPacket::Opcode::RANGED_ATTACK;
			default:
				return OutPacket::Opcode::MAGIC_ATTACK;
			}
		}
	};

	// Tells the server that the player took damage.
	// Opcode: TAKE_DAMAGE(48)
	class TakeDamagePacket : public OutPacket
	{
	public:
		enum From : int8_t
		{
			TOUCH = -1,
			MIST = -4
		};

		// From a mob's touch or attack (0x9593f7): then Power Guard's or
		// Mana Reflection's percent, and the Guardian byte, always 0 here.
		// With either come 14 bytes for the others to show it: whether it
		// was Power Guard (a touch), the mob, its hit action (hit1), and
		// where it and the player were. The last byte says whether Power Stance
		// kept the player in place.
		TakeDamagePacket(const MobAttackResult& result) : OutPacket(OutPacket::Opcode::TAKE_DAMAGE)
		{
			write_time();
			write_byte(result.index);
			write_byte(result.element);
			write_int(result.damage);
			write_int(result.mobid);
			write_int(result.oid);
			write_byte(result.direction);
			write_byte(result.reflect);
			write_byte(0);

			if (result.reflect)
			{
				write_byte(result.index < 0);
				write_int(result.oid);
				write_byte(6);
				write_point(result.mobposition);
				write_point(result.position);
			}

			write_byte(result.stance);
		}

		// From a mob's mist, named by its skill and level (`code`) in place
		// of a mob (FUN_00431783), whose last byte is 2; or from Stop
		// Motion, element 4 with no code and a last byte of 3 (0x94b1d7).
		TakeDamagePacket(uint8_t element, int32_t damage, uint16_t code, uint8_t last = 2) : OutPacket(OutPacket::Opcode::TAKE_DAMAGE)
		{
			write_time();
			write_byte(MIST);
			write_byte(element);
			write_int(damage);
			write_short(static_cast<int16_t>(code));
			write_byte(last);
		}
	};

	// Packet which notifies the server of a skill usage.
	// Opcode: USE_SKILL(91)
	class UseSkillPacket : public OutPacket
	{
	public:
		// What some skills add (0x96d5e5).
		struct Reach
		{
			// Where the player stands, for the skills that send it.
			std::optional<Point<int16_t>> position;
			// The stars Shadow Stars spends.
			int32_t star = 0;
			// The party members reached, if any.
			uint8_t party = 0;
			// The mobs reached, for the skills that reach them.
			std::optional<std::vector<int32_t>> mobs;
		};

		// It ends with how long the skill's action keeps the player from
		// acting (0x96d67a).
		UseSkillPacket(int32_t skillid, int32_t level, uint16_t lock, const Reach& reach) : OutPacket(OutPacket::Opcode::USE_SKILL)
		{
			write_time();
			write_int(skillid);
			write_byte(static_cast<uint8_t>(level));

			if (reach.position)
			{
				write_short(reach.position->x());
				write_short(reach.position->y());
			}

			if (skillid == skill_use::SHADOW_STARS)
				write_int(reach.star);

			if (reach.party != 0)
			{
				write_byte(reach.party);

				// Dispel sends the lock here too.
				if (skillid == skill_use::DISPEL)
					write_short(static_cast<int16_t>(lock));
			}

			if (reach.mobs)
			{
				write_byte(static_cast<uint8_t>(reach.mobs->size()));

				for (int32_t oid : *reach.mobs)
					write_int(oid);
			}

			write_short(static_cast<int16_t>(lock));
		}
	};

	// Opcode: USE_SKILL(91)
	class SummonSkillPacket : public OutPacket
	{
	public:
		// Cast a summon where it should appear, facing as the player
		// (FUN_0096b140). Unlike the other skills it ends with no lock.
		SummonSkillPacket(int32_t skillid, int32_t level, Point<int16_t> position, bool left) : OutPacket(OutPacket::Opcode::USE_SKILL)
		{
			write_time();
			write_int(skillid);
			write_byte(static_cast<uint8_t>(level));
			write_point(position);
			write_byte(left);
		}
	};

	// Opcode: USE_SKILL(91)
	class DoorSkillPacket : public OutPacket
	{
	public:
		// Open a Mystic Door where the player stands (FUN_00969bf9), with no
		// lock after.
		DoorSkillPacket(int32_t skillid, int32_t level, Point<int16_t> position) : OutPacket(OutPacket::Opcode::USE_SKILL)
		{
			write_time();
			write_int(skillid);
			write_byte(static_cast<uint8_t>(level));
			write_point(position);
		}
	};

	// Opcode: SUMMON_ATTACK(176)
	class SummonAttackPacket : public OutPacket
	{
	public:
		// One hit on a mob.
		struct Hit
		{
			int32_t oid;
			int32_t mobid;
			Point<int16_t> position;
			bool mobleft;
			int16_t delay;
			int32_t damage;
		};

		// The player's summon attacked with its `action` (0x7a57d4): where
		// the player and it are, then each mob it hit once.
		SummonAttackPacket(int32_t oid, uint8_t action, bool left, Point<int16_t> owner, Point<int16_t> summon, const std::vector<Hit>& hits) : OutPacket(OutPacket::Opcode::SUMMON_ATTACK)
		{
			write_int(oid);
			write_time();
			write_byte(static_cast<int8_t>((left ? 0x80 : 0) | (action & 0x7F)));
			write_byte(static_cast<int8_t>(hits.size()));
			write_point(owner);
			write_point(summon);

			for (const Hit& hit : hits)
			{
				write_int(hit.oid);
				write_int(hit.mobid);
				// Its hit action, facing, and the damage stat.
				write_byte(6);
				write_byte(static_cast<int8_t>(hit.mobleft ? 0x80 : 0));
				write_byte(0);
				write_byte(0);
				write_point(hit.position);
				write_point(hit.position);
				write_short(hit.delay);
				write_int(hit.damage);
			}

			// The skill data's checksum.
			write_int(0);
		}
	};

	// A mob touched the player's Puppet for `damage` (0x7a627a): attack -1
	// is its body, and `left` whether it came from the left.
	// Opcode: DAMAGE_SUMMON(177)
	class DamageSummonPacket : public OutPacket
	{
	public:
		DamageSummonPacket(int32_t oid, int32_t damage, int32_t mobid, bool left) : OutPacket(OutPacket::Opcode::DAMAGE_SUMMON)
		{
			write_int(oid);
			write_byte(-1);
			write_int(damage);
			write_int(mobid);
			write_byte(left);
		}
	};

	// The player's Beholder heals them (Aura of the Beholder, `hexed` -1)
	// or buffs Hex stat `hexed` (FUN_007a5ab4 / FUN_007a5c99), playing
	// skill1, or skill2 on.
	// Opcode: BEHOLDER(178)
	class BeholderPacket : public OutPacket
	{
	public:
		BeholderPacket(int32_t oid, bool left, int8_t hexed) : OutPacket(OutPacket::Opcode::BEHOLDER)
		{
			write_int(oid);
			write_int(hexed < 0 ? 1320008 : 1320009);
			write_byte(static_cast<int8_t>(left << 7 | (hexed < 0 ? 6 : 7 + hexed)));

			if (hexed >= 0)
				write_byte(hexed);
		}
	};

	// Tells the server which mobs Monster Magnet reached: each one's oid, 0
	// for one out of sight, and whether it was caught (0x96c62a). It may
	// reach none.
	// Opcode: USE_SKILL(91)
	class MonsterMagnetPacket : public OutPacket
	{
	public:
		MonsterMagnetPacket(int32_t skillid, int32_t level, const std::vector<std::pair<int32_t, bool>>& mobs, bool toleft) : OutPacket(OutPacket::Opcode::USE_SKILL)
		{
			write_time();
			write_int(skillid);
			write_byte(static_cast<uint8_t>(level));
			write_int(static_cast<int32_t>(mobs.size()));

			for (auto& [oid, caught] : mobs)
			{
				write_int(oid);
				write_byte(caught);
			}

			write_byte(toleft);
		}
	};

	// Tells the server the player threw a grenade from where they stand,
	// held for `charge` ms (0x95efc5).
	// Opcode: THROW_GRENADE(109)
	class ThrowGrenadePacket : public OutPacket
	{
	public:
		ThrowGrenadePacket(Point<int16_t> position, int32_t charge, int32_t skillid) : OutPacket(OutPacket::Opcode::THROW_GRENADE)
		{
			write_int(position.x());
			write_int(position.y());
			write_int(charge);
			write_int(skillid);
		}
	};

	// Tells the server the player started holding down a skill, or timing
	// one, so others see it (FUN_0096a86e).
	// Opcode: SKILL_EFFECT(93)
	class PrepareSkillPacket : public OutPacket
	{
	public:
		PrepareSkillPacket(int32_t skillid, int32_t level, uint16_t action_and_facing, int8_t speed) : OutPacket(OutPacket::Opcode::SKILL_EFFECT)
		{
			write_int(skillid);
			write_byte(static_cast<uint8_t>(level));
			write_short(static_cast<int16_t>(action_and_facing));
			write_byte(speed);
		}
	};

	// Ends one of the player's skill buffs, as right-clicking its icon does
	// (FUN_0096d873).
	// Opcode: CANCEL_BUFF(92)
	class CancelBuffPacket : public OutPacket
	{
	public:
		CancelBuffPacket(int32_t skillid) : OutPacket(OutPacket::Opcode::CANCEL_BUFF)
		{
			write_int(skillid);
		}
	};

	// Ends one of the player's item buffs, as right-clicking its icon does
	// (FUN_00a096af). The id is the buff's, minus the item id.
	// Opcode: CANCEL_ITEM_EFFECT(73)
	class CancelItemEffectPacket : public OutPacket
	{
	public:
		CancelItemEffectPacket(int32_t buffid) : OutPacket(OutPacket::Opcode::CANCEL_ITEM_EFFECT)
		{
			write_int(buffid);
		}
	};
}