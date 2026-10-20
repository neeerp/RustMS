#pragma once

#include <algorithm>
#include <cstdint>
#include <string>

// Riding a tamed monster (Monster Rider) as v83 does it (report 201).
namespace ms::mount
{
	// The skills that ride a 190xxxx mount: Monster Rider for explorers, Cygnus Knights, Aran and Evan
	// (FUN_00966f7a).
	constexpr int32_t RIDER_SKILLS[] = { 1004, 10001004, 20001004, 20011004 };

	inline bool is_rider_skill(int32_t skill)
	{
		return std::find(std::begin(RIDER_SKILLS), std::end(RIDER_SKILLS), skill) != std::end(RIDER_SKILLS);
	}

	// A tamed monster (190xxxx) rides with a saddle; a vehicle (193xxxx), such as the Battleship, rides
	// alone.
	inline bool is_tamed_monster(int32_t item)
	{
		return item / 10000 == 190;
	}

	inline bool is_vehicle(int32_t item)
	{
		return item / 10000 == 193;
	}

	// The Monster Rider skill whose effect plays as a job gets on or off (FUN_00936859): Evan's base
	// is 2001, everyone else's their job's thousands.
	inline int32_t rider_skill(int32_t job)
	{
		int32_t base = job / 100 == 22 || job == 2001 ? 2001 : job / 1000 * 1000;

		return base * 10000 + 1004;
	}

	// A mount this tired stands tired (FUN_00941eed).
	inline bool tired(int32_t level, int32_t fatigue)
	{
		return fatigue >= level / 3 * 10 + 95;
	}

	// How long after getting on or off the avatar changes, in ms: 30 off each 30 ms update from 240
	// (FUN_00936859, 0x932a2c).
	constexpr int16_t SWITCH_DELAY = 240;

	// How long the client waits for the server to answer getting on or off before it gives up, in ms
	// (0x94aa5e). Until then it uses no skill.
	constexpr int32_t PENDING_TIMEOUT = 7000;

	// A mount's speed and jump, from its TamingMob.nx template and the saddle's bonus (FUN_0094d3d9).
	// The rider's own are not used.
	inline int32_t speed(int32_t base, int32_t bonus)
	{
		return std::min(std::max(base, 80) + bonus, 190);
	}

	inline int32_t jump(int32_t base, int32_t bonus)
	{
		return std::min(std::max(base, 80) + bonus, 123);
	}

	// Whether a mount climbs ladders and ropes: every tamed monster, and some event vehicles.
	inline bool climbs(int32_t mount)
	{
		return is_tamed_monster(mount) || (mount >= 1932003 && mount <= 1932005) ||
			(mount >= 1932007 && mount <= 1932011);
	}

	// The mount's action for the rider's (FUN_004138be): alert stands, and anything else it has no
	// frames for, such as an attack, stands too. A tired mount stands tired.
	inline std::string action(const std::string &rider, bool tired, int32_t mount)
	{
		std::string name = rider;

		bool basic = name == "walk1" || name == "walk2" || name == "stand1" || name == "stand2" ||
			name == "jump" || name == "prone" || name == "fly";
		bool climbing = (name == "ladder" || name == "rope") && climbs(mount);

		if (!basic && !climbing)
			name = "stand1";

		if (tired && (name == "stand1" || name == "stand2"))
			return "tired";

		return name;
	}

	// Where a part of a mount or saddle goes, by its z: in front of the rider, behind, or neither
	// (FUN_00401de9). Base.wz's zmap puts them front to back, the rider between backSaddleFront and
	// saddleRear.
	constexpr const char *ZMAP[] = {
		"mobEquipFront", "tamingMobFront", "mobEquipMid", "saddleFront", "mobEquipUnderSaddle", "tamingMobMid",
		"saddleMid", "backSaddleFront", "saddleRear", "tamingMobRear", "mobEquipRear", "backMobEquipFront",
		"backTamingMobFront", "backMobEquipMid", "backSaddle", "backMobEquipUnderSaddle", "backTamingMobMid"
	};

	// The first of ZMAP behind the rider.
	constexpr int8_t BEHIND = 8;

	// The part's place in ZMAP, the lowest in front, or -1 for none.
	inline int8_t z_index(const std::string &z)
	{
		for (int8_t i = 0; i < static_cast<int8_t>(std::size(ZMAP)); i++)
			if (z == ZMAP[i])
				return i;

		return -1;
	}

	// The client keeps count of the answers it waits for after asking to get on or off: while any is
	// due, no skill goes (FUN_0095f065).
	class Pending
	{
	public:
		// The player asked, at `now` ms.
		void asked(int64_t now)
		{
			if (count == 0)
				since = now;

			count++;
		}

		// A GIVE_BUFF or CANCEL_BUFF came with riding's bit.
		void answered()
		{
			if (count > 0)
				count--;
		}

		// True while any answer is due, until PENDING_TIMEOUT after the first was asked for.
		bool waiting(int64_t now)
		{
			if (count > 0 && now - since >= PENDING_TIMEOUT)
				count = 0;

			return count > 0;
		}

	private:
		int32_t count = 0;
		int64_t since = 0;
	};
}
