#pragma once

#include <algorithm>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

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

	// The skills not used from a ladder or rope (FUN_00969e21): dashes, Ninja Ambush, the Battleship and the
	// event mounts. Monster Rider is.
	inline bool is_ground_only(int32_t skill)
	{
		switch (skill)
		{
			case 1013:
			case 1014:
			case 1017:
			case 1018:
			case 1019:
			case 4121004:
			case 4221004:
			case 5001005:
			case 5221006:
			case 10001014:
			case 10001015:
			case 10001019:
			case 10001022:
			case 10001023:
			case 15001003:
			case 20001019:
			case 20001022:
			case 20001023:
			case 22131001:
			case 22181003:
				return true;
			default:
				break;
		}

		// 1027 to 1031, for each of the four beginners.
		int32_t base = skill % 10000;
		int32_t job = skill / 10000;

		return base >= 1027 && base <= 1031 && (job == 0 || job == 1000 || job == 2000 || job == 2001);
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

	// Whether a saddle (191xxxx) fits a tamed monster, given the monsters its TamingMob.nx img lists
	// (FUN_004b1113). v83 keeps the list as a 64-bit mask of each id's last two digits, so a monster
	// fits any saddle listing one with the same, and one ending 64 or above fits none.
	inline bool saddle_fits(int32_t saddle, int32_t monster, const std::vector<int32_t> &listed)
	{
		if (saddle / 10000 != 191 || !is_tamed_monster(monster) || monster % 100 >= 64)
			return false;

		return std::any_of(listed.begin(), listed.end(), [&](int32_t id)
		{
			return is_tamed_monster(id) && id % 100 == monster % 100;
		});
	}

	// The Corsair's Battleship, and the vehicle it rides (decomp.c:1178699).
	constexpr int32_t BATTLESHIP = 5221006;
	constexpr int32_t BATTLESHIP_VEHICLE = 1932000;
	// The id SKILL_COOLDOWN gives the ship's HP under.
	constexpr int32_t BATTLESHIP_HP = 5221999;

	// The vehicles event skills ride (FUN_008c466d), which the Battleship's rules leave alone.
	inline bool is_event_vehicle(int32_t item)
	{
		return (item >= 1932001 && item <= 1932005) || (item >= 1932007 && item <= 1932011);
	}

	// A vehicle the Battleship's rules hold for.
	inline bool is_ship(int32_t item)
	{
		return is_vehicle(item) && !is_event_vehicle(item);
	}

	// The skills used from the ship (FUN_00966f7a): the Battleship's, the Corsair's guns and buffs, and a
	// few other jobs'. The Battleship itself gets off.
	inline bool ship_allows(int32_t skill)
	{
		switch (skill)
		{
			case 5201002:
			case 5201003:
			case 5211001:
			case 5211002:
			case 5211004:
			case 5211005:
			case 5220002:
			case 5221000:
			case 5221007:
			case 5221008:
			case 5221010:
			case 14111006:
			case 21121008:
			case 22171004:
				return true;
			default:
				return false;
		}
	}

	// Battleship Cannon and Torpedo, used only from the ship (FUN_00969a57).
	inline bool ship_only(int32_t skill)
	{
		return skill == 5221007 || skill == 5221008;
	}

	// How much the ship holds when it is new (FUN_007665f1). The server keeps count.
	inline int32_t battleship_max_hp(int32_t level, int32_t slv)
	{
		return (level - 120 + 2 * slv) * 200;
	}

	// The Monster Rider skill whose effect plays as a job gets on or off (FUN_00936859): Evan's base
	// is 2001, everyone else's their job's thousands.
	inline int32_t rider_skill(int32_t job)
	{
		int32_t base = job / 100 == 22 || job == 2001 ? 2001 : job / 1000 * 1000;

		return base * 10000 + 1004;
	}

	// The skill whose effect plays as `mount` is got on or off (FUN_00936859): the rider's Monster
	// Rider for a tamed monster, the Battleship for the ship, none for anything else.
	inline int32_t ride_effect_skill(int32_t mount, int32_t job)
	{
		if (is_tamed_monster(mount))
			return rider_skill(job);

		return mount == BATTLESHIP_VEHICLE ? BATTLESHIP : 0;
	}

	// A mount this tired stands tired (FUN_00941eed).
	inline bool tired(int32_t level, int32_t fatigue)
	{
		return fatigue >= level / 3 * 10 + 95;
	}

	// How long after getting on or off the avatar changes, in ms: 30 off each 30 ms update from 240
	// (FUN_00936859, 0x932a2c).
	constexpr int16_t SWITCH_DELAY = 240;

	// The Battleship's, which plays its own effect (FUN_00936859).
	constexpr int16_t SHIP_SWITCH_DELAY = 540;

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

	// The actions a vehicle has for the ship's skills (FUN_004138be): Gun Booster's, Maple Warrior's,
	// Flamethrower's, Ice Splitter's, and the Cannon's and Torpedo's.
	inline bool is_ship_action(const std::string &name)
	{
		return name == "alert2" || name == "alert3" || name == "fireburner" || name == "coolingeffect" ||
			name == "cannon" || name == "torpedo";
	}

	// The mount's action for the rider's (FUN_004138be): alert stands, and anything else it has no
	// frames for, such as an attack, stands too. A tired mount stands tired.
	inline std::string action(const std::string &rider, bool tired, int32_t mount)
	{
		std::string name = rider;

		bool basic = name == "walk1" || name == "walk2" || name == "stand1" || name == "stand2" ||
			name == "jump" || name == "prone" || name == "fly";
		bool climbing = (name == "ladder" || name == "rope") && climbs(mount);
		bool shooting = is_vehicle(mount) && is_ship_action(name);

		if (!basic && !climbing && !shooting)
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

	// How the character info window shrinks a rider and moves them in its taming box: by `scale` percent,
	// `x` and `y` from where the rider stands.
	struct InfoPortrait
	{
		int32_t scale;
		int16_t x;
		int16_t y;
	};

	// The mounts too big for the box, fixed in the client (FUN_00903e24). The rest use their item's
	// info/userInfoScale, userInfoDiffX and userInfoDiffY.
	inline std::optional<InfoPortrait> fixed_info_portrait(int32_t mount)
	{
		switch (mount)
		{
			case 1902002:
				return InfoPortrait{ 70, -25, -25 };
			case 1902015:
				return InfoPortrait{ 80, 0, -12 };
			case 1902016:
			case 1902017:
				return InfoPortrait{ 80, -5, -15 };
			case 1902018:
				return InfoPortrait{ 75, -5, -20 };
			default:
				return std::nullopt;
		}
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
