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

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <functional>
#include <optional>
#include <utility>

#include "../../Template/Point.h"

// How summons move, act, and hurt, without the map (v83 CSummoned and
// CVecCtrlSummoned, report 158).
namespace ms::summon_rules
{
	constexpr int32_t OCTOPUS = 5211001;
	constexpr int32_t GAVIOTA = 5211002;
	constexpr int32_t WRATH_OF_THE_OCTOPI = 5220002;
	constexpr int32_t AURA_OF_THE_BEHOLDER = 1320008;
	constexpr int32_t HEX_OF_THE_BEHOLDER = 1320009;

	// How a summon moves: its spawn's moveAbility.
	enum MoveAbility : uint8_t
	{
		STATIONARY = 0,
		FOLLOW = 1,
		WALK = 2,
		CIRCLE_FOLLOW = 3,
		CIRCLE_STATIONARY = 4
	};

	// What it does on its own: its spawn's assistType.
	enum Assist : uint8_t
	{
		NOTHING = 0,
		ATTACK = 1,
		SUPPORT = 2
	};

	// Its animations under `summon` (table 0xbec3bc).
	enum Action : int8_t
	{
		NONE = -1,
		STAND = 0,
		MOVE = 1,
		FLY = 2,
		SUMMONED = 3,
		ATTACK1 = 4,
		ATTACK2 = 5,
		SKILL1 = 6,
		HIT = 12,
		DIE = 13
	};

	inline const char *action_name(int8_t action)
	{
		static const char *names[] = {
				"stand", "move", "fly", "summoned", "attack1", "attack2", "skill1", "skill2", "skill3", "skill4",
				"skill5", "skill6", "hit", "die", "say"
		};

		return action >= 0 && action < 15 ? names[action] : "stand";
	}

	// The skills that summon (net::summons::kind on the server).
	inline bool is_summon(int32_t skill)
	{
		switch (skill)
		{
		case 3111002: case 3211002: case 13111004: case OCTOPUS: case WRATH_OF_THE_OCTOPI:
		case 3111005: case 3211005: case 2311006: case 3121006: case 3221005: case GAVIOTA:
		case 1321007: case 2121005: case 2221005: case 2321003: case 11001004: case 12001004:
		case 13001004: case 14001005: case 15001004: case 12111004:
			return true;
		default:
			return false;
		}
	}

	// The summons that hit one mob, then spread to more around it.
	inline bool spreads(int32_t skill)
	{
		return skill == 3121006 || skill == 3221005 || skill == GAVIOTA || skill == 11001004 ||
			   skill == 12001004 || skill == 13001004 || skill == 14001005 || skill == 15001004;
	}

	// The Puppets: the only summons a mob can hurt, and that a mob turns
	// on.
	inline bool is_puppet(int32_t skill)
	{
		return skill == 3111002 || skill == 3211002 || skill == 13111004;
	}

	// The Octopi, the only summons that fire a ball (FUN_00766612).
	inline bool is_octopus(int32_t skill)
	{
		return skill == OCTOPUS || skill == WRATH_OF_THE_OCTOPI;
	}

	// When a summon's hit lands, after its attackAfter (0x7a55d4): the
	// spreading kinds' all 120 ms later, the others' 50 ms apart; a ball's
	// lands when it arrives instead.
	inline int32_t hit_after(int32_t skill, size_t index)
	{
		return spreads(skill) ? 120 : static_cast<int32_t>(50 * index);
	}

	// Where an Octopus's ball leaves it: its `sp`, drawn facing left, so
	// mirrored when it faces right (0x7a5495).
	inline Point<int16_t> muzzle(Point<int16_t> position, Point<int16_t> sp, bool left)
	{
		return { static_cast<int16_t>(position.x() + (left ? sp.x() : -sp.x())),
				 static_cast<int16_t>(position.y() + sp.y()) };
	}

	// How long the ball flies from `from` to `to`: 600 ms per `speed` px
	// (0x7a54ec), cut to a whole ms.
	inline int32_t ball_time(Point<int16_t> from, Point<int16_t> to, int32_t speed)
	{
		double dx = to.x() - from.x();
		double dy = to.y() - from.y();

		return static_cast<int32_t>(std::sqrt(dx * dx + dy * dy) * 600.0 / std::max(speed, 1));
	}

	// How far the ball is turned, clockwise in radians, to point from
	// `from` to `to`. Its art points left and is mirrored to fly right
	// (0x7a46ac: v83 turns it by atan2(-dy, -dx) and mirrors it past 90°).
	inline float ball_angle(Point<int16_t> from, Point<int16_t> to)
	{
		double dx = to.x() - from.x();
		double dy = to.y() - from.y();

		return static_cast<float>(dx > 0 ? std::atan2(dy, dx) : std::atan2(-dy, -dx));
	}

	// The Cygnus Knights' souls, which follow closer and spread their hits.
	inline bool is_soul(int32_t skill)
	{
		return skill == 11001004 || skill == 12001004 || skill == 13001004 || skill == 14001005 ||
			   skill == 15001004;
	}

	// The mage summons: magic hits, in a box beside them.
	inline bool is_magic(int32_t skill)
	{
		return skill == 2121005 || skill == 2221005 || skill == 2321003 || skill == 12111004 || skill == 2311006 ||
			   skill == 12001004;
	}

	inline bool aims_at_box(int32_t skill)
	{
		return skill == 2121005 || skill == 2221005 || skill == 2321003 || skill == 12111004;
	}

	inline bool flies(uint8_t moves)
	{
		return moves == CIRCLE_FOLLOW || moves == CIRCLE_STATIONARY;
	}

	// Whether it attacks or supports at all (FUN_00766612): not a Puppet.
	inline bool can_act(uint8_t moves, int32_t skill)
	{
		return moves != STATIONARY || is_octopus(skill);
	}

	// The ms between its attacks (FUN_00765047).
	inline int32_t attack_interval(int32_t skill)
	{
		return skill == WRATH_OF_THE_OCTOPI || skill == GAVIOTA ? 1500 : 3000;
	}

	// Whether it attacks while its owner rides `vehicle`, 0 for none (FUN_007a4d42): on a 190xxxx or
	// 193xxxx mount only the Octopi and Gaviota do.
	inline bool attacks_riding(int32_t vehicle, int32_t skill)
	{
		int32_t kind = vehicle / 10000;

		return (kind != 190 && kind != 193) || is_octopus(skill) || skill == GAVIOTA;
	}

	// The animation for a move action (`stance << 1 | left`), unless a
	// one-shot plays (FUN_007a3d98).
	inline Action pick(int8_t oneshot, uint8_t action)
	{
		if (oneshot >= 0)
			return static_cast<Action>(oneshot);

		switch (action >> 1)
		{
		case 1:
			return MOVE;
		case 6:
			return FLY;
		default:
			return STAND;
		}
	}

	// v83 moves a summon every 30 ms physics step.
	constexpr double TICK = 30.0;
	// How fast a follower glides: 7 px a step.
	constexpr double GLIDE = 7.0 / TICK;
	// How fast a flier flies, in px per ms (I: the vecctrl's fly speed).
	constexpr double FLY_SPEED = 0.2;

	// Whether an owner this far away leaves their summon behind, so it
	// moves to them at once.
	inline bool lost(double dx, double dy)
	{
		return std::abs(dx) > 300 || std::abs(dy) > 200;
	}

	struct Follower
	{
		double x;
		double y;
		bool left;
		bool moving = false;
		// How long the owner has stood more than 20 px above or below.
		double apart = 0;
		// Whether it climbs to the owner's height.
		bool climbing = false;
	};

	// Glide `ms` ms after the owner, staying `gap` px from them
	// (FUN_009c7f9b moveAbility 1). An owner on a ladder or rope holds it.
	inline void follow(Follower &summon, double ox, double oy, bool held, double gap, double ms)
	{
		summon.left = ox <= summon.x;
		summon.moving = false;
		double step = GLIDE * ms;

		if (!held)
		{
			if (ox > summon.x + gap)
			{
				summon.x = std::min(ox - gap, summon.x + step);
				summon.moving = true;
			} else if (ox < summon.x - gap)
			{
				summon.x = std::max(ox + gap, summon.x - step);
				summon.moving = true;
			}
		}

		double dy = oy - summon.y;

		if (!summon.climbing)
		{
			summon.apart = std::abs(dy) > 20 ? summon.apart + ms : 0;
			summon.climbing = std::abs(dy) > 100 || summon.apart >= 30 * TICK;
		}

		if (summon.climbing)
		{
			summon.y = dy > 0 ? std::min(oy, summon.y + step) : std::max(oy, summon.y - step);

			if (summon.y == oy)
			{
				summon.climbing = false;
				summon.apart = 0;
			}
		}
	}

	struct Circler
	{
		double x;
		double y;
		bool left;
		double tx = 0;
		double ty = 0;
		// The ms until it picks a new spot.
		double retarget = 0;
	};

	// Fly `ms` ms around `anchor`: the owner (CIRCLE_FOLLOW, FUN_009c857e),
	// or where it was summoned (CIRCLE_STATIONARY), to a spot picked with
	// `rand(n)` in [0, n) that it changes every 1.8-3.6 s or on arriving.
	inline void circle(Circler &summon, double ax, double ay, bool stationary, const std::function<int32_t(int32_t)> &rand,
					   double ms)
	{
		summon.retarget -= ms;

		if (summon.retarget <= 0 || std::hypot(summon.tx - summon.x, summon.ty - summon.y) <= 10)
		{
			if (stationary)
			{
				summon.tx = ax + rand(800) - 400;
				summon.ty = ay + rand(150) - 150;
			} else
			{
				summon.tx = ax + rand(120) - 60;
				summon.ty = ay + rand(60) - 30;
			}

			summon.retarget = (rand(60) + 60) * TICK;
		}

		double dx = summon.tx - summon.x;
		double dy = summon.ty - summon.y;
		double distance = std::hypot(dx, dy);
		double step = std::min(distance, FLY_SPEED * ms);

		if (distance > 0)
		{
			summon.x += dx / distance * step;
			summon.y += dy / distance * step;
		}

		if (dx != 0)
			summon.left = dx < 0;
	}

	// How far ahead of its owner a summon is put: Puppets 200 px, the
	// Octopi 45, the rest where the owner stands.
	inline int16_t placement(int32_t skill)
	{
		if (is_puppet(skill))
			return 200;

		if (is_octopus(skill))
			return 45;

		return 0;
	}

	// Where a summon cast from `owner` facing `left` goes (FUN_0096b140):
	// ahead within the map's x bounds, onto the first foothold within 80
	// px up or down, trying 15 px nearer the owner each time; or nowhere.
	// `ground(x, top)` is the height of the first foothold below (x, top).
	inline std::optional<std::pair<int16_t, int16_t>> place(int32_t skill, int16_t ox, int16_t oy, bool left,
															 int16_t xmin, int16_t xmax,
															 const std::function<int16_t(int16_t, int16_t)> &ground)
	{
		int16_t ahead = placement(skill);

		if (ahead == 0)
			return std::make_pair(ox, oy);

		int32_t direction = left ? -1 : 1;
		int32_t x = std::clamp<int32_t>(ox + direction * ahead, xmin, xmax);

		while ((x - ox) * direction >= 0)
		{
			int16_t y = ground(static_cast<int16_t>(x), static_cast<int16_t>(oy - 80));

			if (y <= oy + 80)
				return std::make_pair(static_cast<int16_t>(x), static_cast<int16_t>(y - 1));

			x -= direction * 15;
		}

		return std::nullopt;
	}

	// One hit of a summon that hits with weapons (FUN_0079216d): 70-100%
	// of the main stat times 2.5, plus the others, times the summon's
	// `pad` percent. `roll` is in [0, 1).
	inline int32_t physical_damage(int32_t main, int32_t others, int32_t pad, double roll)
	{
		double damage = (main * (0.7 + 0.3 * roll) * 2.5 + others) * pad * 0.01;

		return std::max(1, static_cast<int32_t>(damage));
	}

	// What a mob's touch does to a Puppet of skill level `level`
	// (FUN_00793687): its attack `attack` (PADamage and any PAD up) times a
	// roll of 10-85% of itself, times 40 / (level + 70), over 100. `roll` is
	// in [0, 1).
	inline int32_t touch_damage(int32_t attack, int32_t level, double roll)
	{
		double low = attack * 0.1;
		double high = attack * 0.85;

		if (low > high)
			std::swap(low, high);

		double rolled = low + (high - low) * roll;

		return static_cast<int32_t>(40.0 / (level + 70) * attack * rolled * 0.01);
	}

	// How long a Beholder waits after it comes before it first heals or
	// buffs (FUN_007a373a).
	constexpr int64_t SUPPORT_DELAY = 10000;

	// Whether the Beholder's Aura heals its owner by `heal` (FUN_007a5ab4):
	// they are hurt, and if they know Berserk (`berserk` its x %, else 0)
	// and are in it (I: at or under that % of their HP), it wouldn't take
	// them out of it.
	inline bool aura_heals(int32_t hp, int32_t maxhp, int32_t heal, int32_t berserk)
	{
		if (hp >= maxhp)
			return false;

		if (berserk <= 0)
			return true;

		int32_t threshold = maxhp * berserk / 100;

		return hp > threshold || hp + heal <= threshold;
	}

	// Hex of the Beholder's stats, in the order its levels add them.
	enum HexStat : uint8_t
	{
		HEX_PDD,
		HEX_MDD,
		HEX_ACC,
		HEX_EVA,
		HEX_PAD
	};

	// Which stat Hex buffs (FUN_007a5c99), of those its level gives
	// (`hex`, 0 for none): one picked with `roll`, the later ones likelier,
	// unless a lower one is short among the owner's buffs (`have`); -1 with
	// none.
	inline int8_t hex_pick(const std::array<int32_t, 5> &hex, const std::array<int32_t, 5> &have, uint32_t roll)
	{
		int32_t count = 0;
		uint32_t weight = 0;

		for (int32_t i = 0; i < 5; i++)
		{
			if (hex[i] != 0)
			{
				count++;
				weight += i + 1;
			}
		}

		if (count == 0)
			return -1;

		uint32_t r = roll % weight + 1;
		uint32_t reach = 0;
		int32_t picked = 0;

		for (; picked < count; picked++)
		{
			reach += picked + 1;

			if (r <= reach)
				break;
		}

		for (int32_t i = HEX_PAD; i >= HEX_PDD; i--)
			if (hex[i] != 0 && have[i] < hex[i] && picked >= i)
				return static_cast<int8_t>(i);

		return static_cast<int8_t>(picked);
	}

	// The main stat of a summoner's job: STR for warriors, LUK for
	// thieves, DEX for bowmen and pirates, INT for magicians.
	enum MainStat
	{
		STR,
		DEX,
		INT,
		LUK
	};

	inline MainStat main_stat(int16_t job)
	{
		switch (job / 100 % 10)
		{
		case 2:
			return INT;
		case 3:
		case 5:
			return DEX;
		case 4:
			return LUK;
		default:
			return STR;
		}
	}
}
