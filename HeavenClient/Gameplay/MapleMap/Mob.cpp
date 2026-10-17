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
#include "Mob.h"
#include "Layer.h"
#include "SummonRules.h"

#include "../Combat/Heal.h"
#include "../Combat/HitPoint.h"
#include "../Combat/MonsterMagnet.h"
#include "../Combat/Prepare.h"
#include "../../Character/TouchDamage.h"
#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <utility>

namespace ms
{
	Mob::Mob(int32_t oi, int32_t mid, int8_t mode, int8_t st, uint16_t fh, uint16_t homefh, bool newspawn, int8_t tm,
			 Point<int16_t> position, const std::vector<MobStatParser::Status> &spawnstatuses,
			 const Physics &physics) : MapObject(oi), shown(move_path::Player::State{})
	{
		std::string strid = string_format::extend_id(mid, 7);
		nl::node src = nl::nx::mob[strid + ".img"];

		nl::node info = src["info"];

		level = info["level"];
		watk = info["PADamage"];
		matk = info["MADamage"];
		wdef = info["PDDamage"];
		mdef = info["MDDamage"];
		accuracy = info["acc"];
		avoid = info["eva"];
		elements = magic_damage::attributes(std::string(info["elemAttr"]));
		knockback = info["pushed"];
		speed = info["speed"];
		flyspeed = info["flySpeed"];
		chasespeed = info["chaseSpeed"];
		touchdamage = info["bodyAttack"].get_bool();
		undead = info["undead"].get_bool();
		noflip = info["noFlip"].get_bool();
		blocksteleport = info["cantPassByTeleport"].get_bool();
		notattack = info["notAttack"].get_bool();
		firstattack = info["firstAttack"].get_bool();
		showleveltag = !info["hideLevel"].get_bool() && !info["hideHP"].get_bool() &&
					   !info["hideName"].get_bool() && !info["boss"].get_bool() &&
					   (mid / 1000000 != 9 || mid / 100000 == 94 || mid / 100000 == 96);
		canjump = src["jump"].size() > 0;
		canfly = src["fly"].size() > 0;
		canmove = src["move"].size() > 0 || canfly;
		canchase = src["chase"].size() > 0;
		boss = info["boss"].get_bool();
		fixeddamage = info["fixedDamage"];

		if (canfly)
		{
			animations[Stance::STAND] = src["fly"];
			animations[Stance::MOVE] = src["fly"];
		} else
		{
			animations[Stance::STAND] = src["stand"];
			animations[Stance::MOVE] = src["move"];
		}

		if (canchase)
			animations[Stance::CHASE] = src["chase"];

		animations[Stance::JUMP] = src["jump"];
		animations[Stance::HIT] = src["hit1"];
		animations[Stance::DIE] = src["die1"];

		standwidth = animations[Stance::STAND].get_dimensions().x();

		name = std::string(nl::nx::string["Mob.img"][std::to_string(mid)]["name"]);

		nl::node sndsrc = nl::nx::sound["Mob.img"][strid];

		hitsound = sndsrc["Damage"];
		diesound = sndsrc["Die"];

		// info/fs scales both the shoe's acceleration and its drag.
		phobj.shoe = true;
		phobj.walkforce = info["fs"].get_real(1.0);
		phobj.walkdrag = phobj.walkforce;

		id = mid;
		team = tm;
		set_position(position);
		hastarget = false;
		targettime = 0;
		aitime = 0;
		thinktimer = 0;
		movedir = 0;
		jumpdir = 0;
		checkjump = false;
		jumpy = 0;
		phobj.fhid = fh;

		// v83 stands a new mob on the foothold it enters on (FUN_007f115f),
		// even where its position is a little off that foothold, and drops
		// it from where it is without one.
		// A flier enters in the air, and v83 (FUN_005c6499) draws it above
		// every layer.
		if (fh && !canfly)
			physics.get_fht().place_on_fh(phobj, fh);
		else
			phobj.onground = false;

		if (canfly)
			phobj.fhlayer = Layer::Id::SEVEN;

		if (!canfly)
			phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);

		// v83 gives a mob with move but neither jump nor fly move ability 1
		// (0x5d9785), which keeps it off the ends of its platform.
		if (canmove && !canjump && !canfly)
			phobj.set_flag(PhysicsObject::Flag::WALKONLY);

		// One that can also jump gets move ability 2, and keeps near the
		// group of the foothold it spawned on (FUN_005c3ed4).
		if (canmove && canjump && !canfly)
		{
			phobj.set_flag(PhysicsObject::Flag::JUMPER);
			phobj.homegroup = physics.get_fht().get_group(homefh);
		}

		hppercent = 0;
		targetdead = false;
		reporttime = 0;
		dying = false;
		dead = false;
		fading = false;
		stance = Stance::STAND;
		show_stance(st);
		haswaypoint = false;
		flygroup = -1;
		flycount = 0;
		flyx = 0;
		flyy = 0;
		pending_hits = 0;
		death_timeout = 0;

		// It shows where it enters until a path comes, and the AI starts
		// from there.
		shown = move_path::Player({ phobj.crnt_x(), phobj.crnt_y(), 0.0, 0.0,
									static_cast<int16_t>(phobj.onground ? phobj.fhid : 0), value_of(stance, flip) });
		body = phobj;
		bodystance = stance;
		bodyflip = flip;
		update_shoe(false);
		set_statuses(spawnstatuses, false);
		set_control(mode);

		namelabel = Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG,
						 name);
		set_level_tag(0);

		if (newspawn)
		{
			fadein = true;
			opacity.set(0.0f);
		} else
		{
			fadein = false;
			opacity.set(1.0f);
		}
	}

	Mob::Stance Mob::stance_of(uint8_t stancebyte, bool &right) const
	{
		right = (stancebyte % 2) == 0;

		if (!right)
			stancebyte -= 1;

		if (stancebyte < Stance::MOVE)
			stancebyte = Stance::MOVE;

		auto newstance = static_cast<Stance>(stancebyte);

		if (animations.count(newstance) == 0)
			newstance = Stance::MOVE;

		return newstance;
	}

	void Mob::show_stance(uint8_t stancebyte)
	{
		bool right;
		Stance newstance = stance_of(stancebyte, right);

		show_stance(newstance, right);
	}

	void Mob::show_stance(Stance newstance, bool right)
	{
		flip = right;

		if (stance != newstance)
		{
			stance = newstance;

			animations.at(stance).reset();
		}
	}

	void Mob::set_body_stance(Stance newstance)
	{
		if (newstance != Stance::HIT)
		{
			body.clear_flag(PhysicsObject::Flag::NOFRICTION);

			// Monster Magnet's pull ends with the flinch.
			if (pull)
			{
				pull.reset();
				body.hspeed = 0.0;
				body.vspeed = 0.0;
			}
		}

		bodystance = newstance;
	}

	Point<int16_t> Mob::body_position() const
	{
		return { body.get_x(), body.get_y() };
	}

	int8_t Mob::update(const Physics &physics)
	{
		if (!active)
			return phobj.fhlayer;

		bool aniend = animations.at(stance).update();

		if (aniend && stance == Stance::DIE)
			dead = true;

		if (fading)
		{
			opacity -= 0.025f;

			if (opacity.last() < 0.025f)
			{
				opacity.set(0.0f);
				fading = false;
				dead = true;
			}
		} else if (fadein)
		{
			opacity += 0.025f;

			if (opacity.last() > 0.975f)
			{
				opacity.set(1.0f);
				fadein = false;
			}
		}

		if (dead)
		{
			deactivate();

			return -1;
		}

		effects.update();
		headeffects.update();
		fieldeffects.update();

		for (StatusEffect &effect : statuseffects)
			effect.animation.update();
		showhp.update();

		if (!dying)
		{
			for (int32_t number : hurts.update(statuses, fixeddamage, Constants::TIMESTEP))
				hurtnumbers.push_back(number);
		}

		// Every client, controller or not, tells the server once a second how
		// far the local player is from each firstAttack mob (FUN_005ce6dc).
		if (!dying && firstattack)
		{
			if (reporttime > Constants::TIMESTEP)
			{
				reporttime -= Constants::TIMESTEP;
			} else
			{
				reporttime = 1000;

				Point<int16_t> position = get_position();
				int32_t dx = std::abs(targetpos.x() - position.x());
				int32_t dy = std::abs(targetpos.y() - position.y());

				MobDistancePacket(oid, dx / 10 + dy / 3 + (targetdead ? 100 : 0)).dispatch();
			}
		}

		if (!dying)
		{
			if (control)
			{
				// A firstAttack mob that can attack (FUN_005ce8c6) starts
				// chasing the local player once they come within
				// |dx| / 10 + |dy| / 3 <= 40 of it.
				if (firstattack && !notattack && !hastarget)
				{
					Point<int16_t> position = get_position();
					int32_t dx = std::abs(targetpos.x() - position.x());
					int32_t dy = std::abs(targetpos.y() - position.y());

					if (dx / 10 + dy / 3 <= 40)
						chase();
				}

				if (flinch)
				{
					flinch_body(physics);
				} else if (!shown.playing() && ctrlstate >= 3)
				{
					// The next path goes once the last has played and been
					// acked (FUN_006675a8).
					plan(physics);
				}
			}

			shown.step(Constants::TIMESTEP);

			const move_path::Player::State &now = shown.get_state();

			// A hit shows at once, over the path, until it has played.
			if (acting && aniend)
				acting = false;

			if (!acting)
				show_stance(now.stance);

			phobj.hspeed = now.x - phobj.crnt_x();
			phobj.vspeed = now.y - phobj.crnt_y();
			phobj.move();

			if (!canfly)
				physics.get_fht().update_fh(phobj);
		} else
		{
			phobj.normalize();
			physics.get_fht().update_fh(phobj);

			if (death_timeout > 0 && --death_timeout == 0 && stance != Stance::DIE)
				apply_death();
		}

		return phobj.fhlayer;
	}

	void Mob::seat_body(const Physics &physics)
	{
		move_path::Player::State end = shown.end();

		body.set_x(end.x);
		body.set_y(end.y);
		body.hspeed = end.vx * Constants::TIMESTEP / 1000.0;
		body.vspeed = end.vy * Constants::TIMESTEP / 1000.0;

		if (end.fh > 0 && !canfly)
			physics.get_fht().place_on_fh(body, end.fh);
		else
			body.onground = false;

		bool right;
		Stance endstance = stance_of(end.stance, right);

		// It may stop mid-flinch, but only a flinch it takes holds it.
		set_body_stance(endstance == Stance::HIT ? Stance::STAND : endstance);
		bodyflip = right;
		hittime = 0;

		recorder.reset(Movement(Movement::ABSOLUTE, 0, body.get_x(), body.get_y(), Movement::velocity(body.hspeed),
								Movement::velocity(body.vspeed), end.fh, value_of(bodystance, bodyflip), 0));
		seated = true;
	}

	void Mob::plan(const Physics &physics)
	{
		if (!seated)
			seat_body(physics);

		// Up to 55 of v83's 90 ms steps (66bbb8).
		for (int32_t ms = 0; ms < 55 * 90 && !recorder.ready(canfly); ms += Constants::TIMESTEP)
			step_body(physics);

		move_path::Path path = recorder.flush(canfly);

		// It asks to attack when chasing, unless controlling, then 70% of
		// the time.
		bool wantattack = hastarget && (ctrlstate < 0 || randomizer.next_int(100) < 70);
		int8_t flags = (ctrlstate < 0 ? 0x10 : 0) | (wantattack ? 1 : 0);

		MoveMobPacket(oid, ++moveid, flags, -1, false, 0, canfly, path).dispatch();

		// It plays what it sends, as others will (FUN_0068a88d).
		shown.receive(path.elements);

		ctrlstate = ctrlstate == -2 || ctrlstate == 1 ? 1 : 2;
	}

	void Mob::flinch_body(const Physics &physics)
	{
		// v83 stops what it shows at the next foothold and starts the AI
		// again from there (FUN_0068b16f, FUN_009bbd3d). Only walkers and
		// jumpers go on to one.
		shown.land(canfly || !canmove);
		seat_body(physics);

		Point<int16_t> position = body_position();

		bodyflip = flinch->user ? flinch->user->x() > position.x() : flinch->left;
		set_body_stance(Stance::HIT);
		hittime = animations.at(Stance::HIT).get_duration();

		// It stops steering while hit, so a flier dives (FUN_007f7f02).
		flyx = 0;
		flyy = 0;

		if (flinch->user)
		{
			// Monster Magnet pulls it in, then it chases the user (0x66b562).
			int16_t reach = standwidth + magnet::REACH;

			pull = Pull{ *flinch->user, reach, magnet::start_speed(position.x(), flinch->user->x(), reach), 0 };
			body.set_flag(PhysicsObject::Flag::NOFRICTION);
			chase();
		} else if (canmove)
		{
			// v83 pushes a flinching mob away from its attacker at 130 px/s.
			constexpr double KBSPEED = 130.0 * Constants::TIMESTEP / 1000.0;

			body.hspeed = bodyflip ? -KBSPEED : KBSPEED;
			body.set_flag(PhysicsObject::Flag::NOFRICTION);
		}

		flinch.reset();
		plan(physics);
	}

	void Mob::step_body(const Physics &physics)
	{
		// A stunned, frozen or webbed mob stands where it is, and its AI
		// waits (inferred: the hold itself wasn't traced in v83).
		bool hold = held() && bodystance != Stance::HIT && bodystance != Stance::JUMP;

		if (hold)
		{
			if (bodystance != Stance::STAND)
				set_body_stance(Stance::STAND);

			flyx = 0;
			flyy = -1;
		}

		if (!canfly)
		{
			if (body.is_flag_not_set(PhysicsObject::Flag::TURNATEDGES))
			{
				// A knocked back mob stops at the edge, still facing its
				// attacker.
				if (bodystance != Stance::HIT)
					bodyflip = !bodyflip;

				body.set_flag(PhysicsObject::Flag::TURNATEDGES);
			}
		}

		if (canfly)
		{
			body.walkdir = flyx;
			body.flydir = flyy;
		}

		switch (bodystance)
		{
			case Stance::MOVE:
			case Stance::CHASE:
				if (!canfly)
					body.walkdir = bodyflip ? 1 : -1;

				break;
			case Stance::HIT:
				if (pull)
				{
					// Monster Magnet's pull (0x9bbdfd) slows each 30 ms
					// step once the mob nears the user, and a flier rises
					// or sinks toward them too.
					Point<int16_t> position = body_position();

					pull->elapsed += Constants::TIMESTEP;

					while (pull->elapsed >= magnet::STEP)
					{
						pull->elapsed -= magnet::STEP;
						pull->speed = magnet::step_speed(pull->speed, position.x(), pull->user.x(), pull->reach);
					}

					body.hspeed = pull->speed * Constants::TIMESTEP / 1000.0;

					if (canfly)
						body.vspeed = magnet::fly_speed(position.y(), pull->user.y()) * Constants::TIMESTEP / 1000.0;
				} else if (body.is_flag_set(PhysicsObject::Flag::NOFRICTION) && body.onground)
				{
					// v83 slows the push by 40000 / mass (100) px/s^2
					// along the ground; walls and edges zero it.
					constexpr double KBDECEL = 400.0 * Constants::TIMESTEP * Constants::TIMESTEP / 1000000.0;

					double kbspeed = std::abs(body.hspeed) - KBDECEL;

					if (kbspeed > 0.0)
					{
						body.hspeed = body.hspeed < 0.0 ? -kbspeed : kbspeed;
					} else
					{
						body.hspeed = 0.0;
						body.clear_flag(PhysicsObject::Flag::NOFRICTION);
					}
				}

				break;
			case Stance::JUMP:
				// CVecCtrl::Jump (FUN_007f1bb0) speeds the mob up in the
				// direction it holds, and it keeps steering in the air.
				if (body.onground)
					Physics::jump(body, Physics::jump_speed(1.0), jumpdir);

				body.walkdir = jumpdir;

				break;
		}

		if (body.is_flag_set(PhysicsObject::Flag::JUMPER))
		{
			// At a loose end or a wall (FUN_007f8d64), a jumper jumps unless
			// it is chasing a target below it, and then only half the time.
			// Toward a target on its own group it walks down a step instead.
			const FootholdTree &fht = physics.get_fht();
			bool chasing = hastarget && bodystance != Stance::HIT;

			body.edgejump = !chasing || targetpos.y() <= body.get_y() || randomizer.next_bool();
			body.stepdown = !chasing || fht.get_group_below(targetpos) == fht.get_group(body.fhid);
		}

		physics.move_object(body);

		bool jumprequest = body.jumprequest;
		body.jumprequest = false;

		// v83 gives up a chase 3 to 4 minutes after aggro (FUN_007f7f02).
		if (hastarget)
		{
			targettime -= Constants::TIMESTEP;

			if (targettime < 0)
				hastarget = false;
		}

		if (!hastarget)
			puppet = 0;

		// v83 (FUN_005cc410) picks a walking mob's action as it goes:
		// its chase animation for as long as it has a target.
		if (!canfly && (bodystance == Stance::MOVE || bodystance == Stance::CHASE))
			walk(input());

		switch (bodystance)
		{
			case Stance::HIT:
				// v83 holds a hit mob still for its hit, then leaves it
				// standing until its AI next decides.
				hittime -= Constants::TIMESTEP;

				if (hittime <= 0)
					set_body_stance(Stance::STAND);

				break;
			case Stance::JUMP:
				// It lands still holding the direction it jumped in,
				// and its AI only counts down in the air.
				if (body.onground)
				{
					walk(jumpdir);
				} else
				{
					for (aitime += Constants::TIMESTEP; aitime >= 90; aitime -= 90)
						thinktimer--;
				}

				break;
			default:
				if (hold)
					break;

				// The mob vecctrl steps its AI in 90 ms steps (FUN_007f72d9).
				for (aitime += Constants::TIMESTEP; aitime >= 90; aitime -= 90)
				{
					if (canfly)
						think_fly(physics);
					else
						think(physics);
				}

				break;
		}

		// A jumper that stopped at the end of its foothold jumps on the
		// next tick (FUN_007f1fec).
		if (jumprequest && !hold &&
			(bodystance == Stance::MOVE || bodystance == Stance::CHASE || bodystance == Stance::STAND))
			jump(input(), false);

		// Footholds are recorded only while on them (FUN_009b108a).
		int16_t fh = body.onground ? static_cast<int16_t>(body.fhid) : 0;

		recorder.step(Movement(Movement::ABSOLUTE, 0, body.get_x(), body.get_y(), Movement::velocity(body.hspeed),
							   Movement::velocity(body.vspeed), fh, value_of(bodystance, bodyflip), 0),
					  Constants::TIMESTEP);
	}

	void Mob::think_fly(const Physics &physics)
	{
		// FUN_007f8875 steers for the waypoint unless within 10 px of it
		// across, glides once within 10 px of its height, and beats up while
		// it is higher.
		if (haswaypoint)
		{
			double x = body.crnt_x();
			double y = body.crnt_y();
			int8_t xdir = 0;
			int8_t ydir = 0;

			if (std::abs(static_cast<int32_t>(x - waypoint.x())) > 9)
				xdir = x <= waypoint.x() ? 1 : -1;

			if (std::abs(static_cast<int32_t>(y - waypoint.y())) < 10)
				ydir = -1;
			else if (waypoint.y() < y)
				Physics::fly_jump(body);

			fly(xdir, ydir);

			// It moves on once within 11 px of the waypoint, or once its
			// target is 150 px from it.
			auto within = [&](Point<int16_t> point, int16_t range)
			{
				return point.x() >= waypoint.x() - range && point.x() < waypoint.x() + range &&
					   point.y() >= waypoint.y() - range && point.y() < waypoint.y() + range;
			};

			Point<int16_t> position(static_cast<int16_t>(x), static_cast<int16_t>(y));

			if (within(position, 11) || (hastarget && !within(targetpos, 150)))
				haswaypoint = false;
		}

		if (!haswaypoint)
			pick_waypoint(physics);
	}

	void Mob::pick_waypoint(const Physics &physics)
	{
		// Chasing, it heads up to 120 px across and 40 px up or down from its
		// target, and gives up after 59 to 118 more waypoints.
		if (hastarget)
		{
			if (flycount == 0)
				flycount = randomizer.next_int(60) + 60;

			waypoint = targetpos + Point<int16_t>(randomizer.next_int(240) - 120, randomizer.next_int(80) - 40);
			haswaypoint = true;
			flygroup = -1;

			if (--flycount < 1)
				hastarget = false;
			else
				return;
		}

		// Otherwise it roams a random group for (its width + 40 to 239) / 8
		// waypoints, each from 50 px above to 9 px below one of its platforms.
		const FootholdTree &fht = physics.get_fht();
		const std::vector<uint16_t> &groups = fht.get_groups();

		if (groups.empty())
			return;

		if (flycount < 1 || flygroup < 0)
		{
			flygroup = groups[randomizer.next_int(groups.size())];

			Range<int16_t> reach = fht.get_group_reach(flygroup);

			flycount = (reach.second() - reach.first() + 40 + randomizer.next_int(200)) >> 3;
		}

		const std::vector<uint16_t> &footholds = fht.get_group_footholds(flygroup);
		const Foothold &fh = fht.get_fh(footholds[randomizer.next_int(footholds.size())]);
		double length = fh.length();
		auto span = static_cast<int32_t>(length);
		double along = span > 0 ? randomizer.next_int(span) : 0;
		double ux = length > 0.0 ? fh.hdelta() / length : 0.0;
		double uy = length > 0.0 ? fh.vdelta() / length : 0.0;

		waypoint = {
			static_cast<int16_t>(fh.x1() + ux * along),
			static_cast<int16_t>(fh.y1() + uy * along + randomizer.next_int(60) - 50)
		};
		haswaypoint = true;
		flycount--;
	}

	void Mob::fly(int8_t x, int8_t y)
	{
		flyx = x;
		flyy = y;

		if (x == 0)
		{
			set_body_stance(Stance::STAND);
		} else
		{
			set_body_stance(Stance::MOVE);
			bodyflip = x > 0;
		}
	}

	void Mob::think(const Physics &physics)
	{
		if (body.is_flag_set(PhysicsObject::Flag::JUMPER) && think_jump(physics))
			return;

		// FUN_007f8025 (standing mobs), FUN_007f81c8 (walking) and
		// FUN_007f835e (jumping): each decision holds for a random time, in
		// 90 ms steps.
		if (thinktimer > 0)
		{
			thinktimer--;
			return;
		}

		Point<int16_t> position = body_position();
		int16_t dx = targetpos.x() - position.x();

		if (!canmove)
		{
			if (hastarget)
			{
				thinktimer = (randomizer.next_int(400) + 800) / 90;

				bodyflip = dx >= 0;

				// It forgets a target that leaves the 2000 x 1200 box around it.
				int16_t dy = targetpos.y() - position.y();

				if (dx < -1000 || dx >= 1000 || dy < -600 || dy >= 600)
					hastarget = false;
			}
		} else if (hastarget)
		{
			// Head for the target, and keep going the way it was until 100 px
			// past it. Turning at a platform's end is left to the walk.
			thinktimer = (randomizer.next_int(400) + 800) / 90;

			if (movedir == 0 || std::abs(dx) > 99)
				movedir = dx >= 0 ? 1 : -1;

			walk(movedir);
		} else if (canjump && body.onground && randomizer.next_int(5) == 0)
		{
			// A fifth of a jumping mob's idle decisions are jumps, after
			// which it decides again.
			jump(input(), false);
			return;
		} else
		{
			thinktimer = (randomizer.next_int(2000) + 1000) / 90;
			movedir = static_cast<int8_t>(randomizer.next_int(3) - 1);
			walk(movedir);
		}

		thinktimer--;
	}

	bool Mob::think_jump(const Physics &physics)
	{
		// FUN_007f835e only looks around on the ground.
		if (!body.onground)
		{
			thinktimer--;
			return true;
		}

		int8_t dir = input();

		// A jump it chose that lands at the height it left from turns it
		// around.
		if (checkjump)
		{
			checkjump = false;

			if (dir != 0 && body.get_y() == jumpy)
			{
				dir = -dir;
				walk(dir);
			}
		}

		// It jumps at a rising wall up to 40 px ahead while its decision
		// lasts, or turns back once pressed against it.
		const FootholdTree &fht = physics.get_fht();

		switch (fht.get_wall_ahead(body, dir))
		{
			case FootholdTree::WallAhead::AGAINST:
				dir = -dir;
				walk(dir);
				break;
			case FootholdTree::WallAhead::NEAR:
				if (thinktimer > 0)
				{
					jump(dir, true);
					thinktimer--;
					return true;
				}

				break;
			default:
				break;
		}

		// Chasing, a third of its steps under a target on another group
		// that reaches over it are jumps, after which it decides again.
		if (hastarget && thinktimer > 0)
		{
			uint16_t group = fht.get_group_below(targetpos);
			Range<int16_t> reach = fht.get_group_reach(group);
			int16_t x = body.get_x();

			if (group != fht.get_group(body.fhid) && targetpos.y() < body.get_y() &&
				x > reach.first() && x < reach.second() && randomizer.next_int(3) == 0)
			{
				movedir = 0;
				thinktimer = -1;
				jump(dir, true);
				return true;
			}
		}

		return false;
	}

	void Mob::jump(int8_t dir, bool check)
	{
		jumpdir = dir;
		checkjump = check;
		jumpy = body.get_y();
		set_body_stance(Stance::JUMP);
	}

	int8_t Mob::input() const
	{
		switch (bodystance)
		{
			case Stance::MOVE:
			case Stance::CHASE:
				return bodyflip ? 1 : -1;
			case Stance::JUMP:
				return jumpdir;
			default:
				return 0;
		}
	}

	void Mob::walk(int8_t direction)
	{
		if (direction == 0)
		{
			set_body_stance(Stance::STAND);
		} else
		{
			set_body_stance(hastarget && canchase ? Stance::CHASE : Stance::MOVE);
			bodyflip = direction > 0;
		}
	}

	void Mob::acknowledge(int16_t ackid, bool nextattack)
	{
		if (!control)
			set_control(1);

		if (ackid < moveid)
			ctrlstate = 2;
		else
			ctrlstate = nextattack ? 4 : 3;
	}

	void Mob::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		Point<int16_t> headp = get_head_position(absp);
		Point<int16_t> viewp = Point<int16_t>(static_cast<int16_t>(viewx), static_cast<int16_t>(viewy));

		effects.drawbelow(absp, alpha);
		headeffects.drawbelow(headp, alpha);
		fieldeffects.drawbelow(viewp, alpha);

		if (!dead)
		{
			float interopc = opacity.get(alpha);

			animations.at(stance).draw(DrawArgument(absp, flip && !noflip, interopc), alpha);

			if (showhp)
			{
				namelabel.draw(absp);

				if (showleveltag)
				{
					int16_t width = 0;

					for (const Texture& part : leveltag)
						width += part.width();

					// v83 ends the tag 1 px left of the name box and starts
					// it 2 px below the box's top, level with the text.
					Point<int16_t> tagpos = absp + Point<int16_t>(-namelabel.width() / 2 - width - 3, 8);

					for (const Texture& part : leveltag)
					{
						part.draw(tagpos);
						tagpos.shift_x(part.width());
					}
				}

				if (!dying && hppercent > 0)
					draw_hp_bar(absp);
			}
		}

		if (!dead)
		{
			Rectangle<int16_t> body = get_body();
			Point<int16_t> position = get_position();

			for (const StatusEffect &effect : statuseffects)
			{
				std::optional<hit_point::Placement> placement =
						hit_point::place(effect.pos, body, position, hit_point::centre(body, body));

				if (!placement)
					continue;

				Point<int16_t> at = placement->offset;

				switch (placement->anchor)
				{
					case hit_point::Anchor::FEET:
						at += absp;
						break;
					case hit_point::Anchor::HEAD:
						at += headp;
						break;
					case hit_point::Anchor::FIELD:
						at += viewp;
						break;
				}

				effect.animation.draw(DrawArgument(at, flip && !noflip), alpha);
			}
		}

		effects.drawabove(absp, alpha);
		headeffects.drawabove(headp, alpha);
		fieldeffects.drawabove(viewp, alpha);
	}

	void Mob::draw_hp_bar(Point<int16_t> position) const
	{
		// As in v83 (FUN_005d004e), the bar sits centred over the body, its
		// top 30 px above the body's. A frame without a body has no bar.
		Rectangle<int16_t> body = animations.at(stance).get_bounds();

		if (body.right() <= body.left() || body.bottom() <= body.top())
			return;

		int16_t centre = (body.left() + body.right()) / 2;

		if (flip && !noflip)
			centre = -centre;

		hpbar.draw(position + Point<int16_t>(centre - MobHpBar::WIDTH / 2, body.top() - 30), hppercent);
	}

	void Mob::set_control(int8_t mode)
	{
		// Taking control lets it move at once; it may attack only once a
		// move is acked. Letting go remembers whether it could have.
		// The AI starts where the mob is shown going (FUN_006637ec).
		if (mode > 0 && !control)
		{
			ctrlstate = 3;
			seated = false;
		} else if (mode == 0 && control)
			ctrlstate = ctrlstate == 3 ? -2 : ctrlstate == 4 ? -3 : -1;

		control = mode > 0;

		// v83 (FUN_005d511d) has a mob given with aggro chase the local
		// player.
		if (mode == 2)
		{
			chase();
		} else if (mode == 0)
		{
			hastarget = false;
			flinch.reset();
			update_shoe(false);
		}
	}

	void Mob::chase()
	{
		// FUN_007f73d8
		hastarget = true;
		targettime = 180000 + randomizer.next_int(60000);
		thinktimer = 0;
		movedir = 0;
		update_shoe(true);
	}

	void Mob::update_shoe(bool chasing)
	{
		// v83 mobs walk on a CAttrShoe (FUN_005c55cd): its speed is
		// info/speed + 100 clamped to 10-140 percent. A flier's shoe flies
		// and walks at info/flySpeed instead, the only speed most fliers
		// have. A mob with a chase animation walks at info/chaseSpeed while
		// chasing. v83 sets the shoe when a chase starts, not when one runs
		// out, so the mob keeps that speed until something else resets it.
		// A Speed status (Slow) adds its value to the percentage
		// (FUN_006642b0).
		auto found = statuses.find(MobStatParser::SPEED);
		float change = found != statuses.end() ? found->second.value : 0.0f;

		auto shoespeed = [change](float base)
		{
			return std::clamp(base + 100.0f + change, 10.0f, 140.0f) * 0.01;
		};

		chaseshoe = chasing;

		float base = canfly ? flyspeed : speed;

		body.walkspeed = canmove ? shoespeed(chasing && canchase ? chasespeed : base) : 0.0;

		if (canfly)
			body.flyspeed = shoespeed(base);
	}

	void Mob::set_target(Point<int16_t> position, bool dead)
	{
		targetpos = puppet ? puppetpos : position;
		targetdead = puppet ? false : dead;
	}

	void Mob::aggro_puppet(int32_t oid, Point<int16_t> position)
	{
		if (!control || !hastarget)
			return;

		targettime = 180000 + randomizer.next_int(60000);

		if (puppet)
			return;

		puppet = oid;
		puppetpos = position;
		targetpos = position;
		targetdead = false;
	}

	void Mob::release_puppet(int32_t oid)
	{
		if (puppet != oid)
			return;

		puppet = 0;
		hastarget = false;
	}

	int32_t Mob::roll_puppet_damage(int32_t level) const
	{
		auto found = statuses.find(MobStatParser::PAD);
		int32_t attack = watk + (found != statuses.end() ? static_cast<int32_t>(found->second.value) : 0);

		return summon_rules::touch_damage(attack, level, randomizer.next_real(1.0));
	}

	void Mob::send_movement(Point<int16_t>, std::vector<Movement> &&in_movements)
	{
		// v83 plays the path on from where the mob is shown; its start is
		// not used (FUN_0068b371).
		if (control)
			return;

		shown.receive(in_movements);
	}

	Point<int16_t> Mob::get_head_position(Point<int16_t> position) const
	{
		Point<int16_t> head = animations.at(stance).get_head();

		position.shift_x((flip && !noflip) ? -head.x() : head.x());
		position.shift_y(head.y());

		return position;
	}

	void Mob::kill(int8_t animation)
	{
		switch (animation)
		{
			case 0:
				deactivate();
				break;
			case 1:
				dying = true;

				// The server kills the mob before the swing connects; let the
				// killing hit land first.
				if (pending_hits == 0)
					apply_death();
				else
					death_timeout = 250; // 2 s

				break;
			case 2:
				fading = true;
				dying = true;
				break;
		}
	}

	void Mob::set_level_tag(uint8_t set)
	{
		nl::node src = NxFiles::ui_classic["Basic.img"]["ShowLevel"];
		nl::node digits = src[std::to_string(set)];

		leveltag.clear();
		leveltag.emplace_back(digits["left"]);

		for (char digit : std::to_string(level))
			leveltag.emplace_back(digits[std::string(1, digit)]);

		leveltag.emplace_back(src["Bracket"]["right"]);
	}

	void Mob::expect_hits(int32_t count)
	{
		pending_hits = static_cast<uint16_t>(std::clamp<int32_t>(pending_hits + count, 0, UINT16_MAX));
	}

	void Mob::show_hp(int8_t percent, uint16_t playerlevel)
	{
		// v83 picks the name's font and tag set when it first builds the
		// tag (FUN_0066c734): 20 or more levels above the player is orange,
		// 20 or more below is yellow.
		if (hppercent == 0)
		{
			if (level >= playerlevel + 20)
			{
				namelabel.change_color(Color::Name::STRONGMOB);
				set_level_tag(2);
			}
			else if (level + 20 <= playerlevel)
			{
				namelabel.change_color(Color::Name::WEAKMOB);
				set_level_tag(1);
			}
		}

		if (percent > 100)
			percent = 100;
		else if (percent < 0)
			percent = 0;

		hppercent = percent;
		// v83 hides the bar and name 5 s after the player's last hit
		// (FUN_005cbf42 stamps the hit; the mob's update checks it).
		showhp.set_for(5000);
	}

	void Mob::show_effect(const Animation &animation, int8_t pos, int8_t z, bool f, const Rectangle<int16_t> &range)
	{
		if (!active)
			return;

		Rectangle<int16_t> body = get_body();
		Point<int16_t> position = get_position();
		std::optional<hit_point::Placement> placement =
				hit_point::place(pos, body, position, hit_point::centre(range, body));

		if (!placement)
			return;

		DrawArgument args(placement->offset, f);

		switch (placement->anchor)
		{
			case hit_point::Anchor::FEET:
				effects.add(animation, args, z);
				break;
			case hit_point::Anchor::HEAD:
				headeffects.add(animation, args, z);
				break;
			case hit_point::Anchor::FIELD:
				fieldeffects.add(animation, args, z);
				break;
		}
	}

	void Mob::set_statuses(const std::vector<MobStatParser::Status> &given, bool effects)
	{
		std::vector<int32_t> reasons;

		for (const MobStatParser::Status &status : given)
		{
			statuses[status.stat] = status;
			hurts.start(status.stat);

			if (std::find(reasons.begin(), reasons.end(), status.reason) == reasons.end())
				reasons.push_back(status.reason);

			if (status.stat == MobStatParser::SPEED)
				update_shoe(chaseshoe);

			// Doom shows the mob as a Blue Snail (FUN_00789efd with 100101).
			if (status.stat == MobStatParser::DOOM)
				show_look(100101);
		}

		for (int32_t reason : reasons)
			show_status(reason, effects);
	}

	void Mob::reset_statuses(const BuffParser::Flags &flags)
	{
		for (auto iter = statuses.begin(); iter != statuses.end();)
		{
			if (flags.has(iter->first))
				iter = statuses.erase(iter);
			else
				++iter;
		}

		statuseffects.erase(
				std::remove_if(statuseffects.begin(), statuseffects.end(),
							   [this](const StatusEffect &effect)
							   {
								   return std::none_of(statuses.begin(), statuses.end(),
													   [&effect](const auto &status)
													   {
														   return status.second.reason == effect.reason;
													   });
							   }),
				statuseffects.end());

		if (flags.has(MobStatParser::SPEED))
			update_shoe(chaseshoe);

		// And its own look again (FUN_00789dc7).
		if (flags.has(MobStatParser::DOOM))
			show_look(0);
	}

	std::vector<int32_t> Mob::take_hurts()
	{
		return std::exchange(hurtnumbers, {});
	}

	bool Mob::held() const
	{
		return statuses.count(MobStatParser::STUN) || statuses.count(MobStatParser::FREEZE) ||
			   statuses.count(MobStatParser::WEB);
	}

	void Mob::show_status(int32_t reason, bool effects)
	{
		// A player's skill, or a mob skill's id and level (0x5d3d02).
		nl::node src = nl::nx::skill[string_format::extend_id(reason / 10000, 3) + ".img"]["skill"]
		[string_format::extend_id(reason, 7)];

		if (!src)
			src = nl::nx::skill["MobSkill.img"][string_format::extend_id(reason & 0xFFFF, 3)]["level"]
			[std::to_string(reason >> 16)];

		// Which of the `mob` effects repeat comes from the data (the draw
		// code wasn't traced).
		for (uint8_t part = 0; part < 2; part++)
		{
			nl::node effect = src[part == 0 ? "mob" : "mob0"];

			if (effect.size() == 0)
				continue;

			auto pos = static_cast<int8_t>(effect["pos"].get_integer());
			bool lasts = part == 1 || effect["repeat"].get_bool();

			if (!lasts)
			{
				if (effects)
					show_effect(effect, pos, 0, flip, get_body());

				continue;
			}

			bool shown = std::any_of(statuseffects.begin(), statuseffects.end(),
									 [reason, part](const StatusEffect &other)
									 {
										 return other.reason == reason && other.part == part;
									 });

			if (!shown)
				statuseffects.push_back({ reason, part, effect, pos });
		}
	}

	void Mob::show_look(int32_t mobid)
	{
		if (mobid == 0)
		{
			if (ownlook)
			{
				animations = std::move(*ownlook);
				ownlook.reset();
				animations.at(stance).reset();
			}

			return;
		}

		if (ownlook)
			return;

		nl::node src = nl::nx::mob[string_format::extend_id(mobid, 7) + ".img"];
		std::map<Stance, Animation> look;

		// Every stance the mob takes needs a frame; what the other lacks it
		// stands for.
		for (const auto &own : animations)
		{
			nl::node stancesrc = src[own.first == Stance::CHASE ? "move" : nameof(own.first)];

			look[own.first] = stancesrc.size() > 0 ? stancesrc : src["stand"];
		}

		ownlook = std::move(animations);
		animations = std::move(look);
	}

	void Mob::show_hit(const Animation &animation, const Rectangle<int16_t> &range, bool f)
	{
		if (!active)
			return;

		effects.add(animation, DrawArgument(hit_point::centre(range, get_body()) - get_position(), f), 0);
	}

	float Mob::calculate_hitchance(int16_t leveldelta, int32_t player_accuracy) const
	{
		float faccuracy = static_cast<float>(player_accuracy);
		float hitchance = faccuracy / (((1.84f + 0.07f * leveldelta) * avoid) + 1.0f);

		if (hitchance < 0.01f)
			hitchance = 0.01f;

		return hitchance;
	}

	double Mob::calculate_mindamage(int16_t leveldelta, double damage) const
	{
		double mindamage = damage * (1 - 0.01 * leveldelta) - wdef * 0.6;

		return mindamage < 1.0 ? 1.0 : mindamage;
	}

	double Mob::calculate_maxdamage(int16_t leveldelta, double damage) const
	{
		double maxdamage = damage * (1 - 0.01 * leveldelta) - wdef * 0.5;

		return maxdamage < 1.0 ? 1.0 : maxdamage;
	}

	std::vector<std::pair<int32_t, bool>> Mob::calculate_damage(const Attack &attack)
	{
		double mindamage;
		double maxdamage;
		float hitchance;
		float critical;
		int16_t leveldelta = level - attack.playerlevel;

		if (leveldelta < 0)
			leveldelta = 0;

		Attack::DamageType damagetype = attack.damagetype;

		std::vector<std::pair<int32_t, bool>> result(attack.hitcount);

		// A spell rolls by v83's own formula, and never crits.
		if (damagetype == Attack::DamageType::DMG_MAGIC)
		{
			for (auto &hit : result)
				hit = {next_magic_damage(attack), false};

			return result;
		}

		switch (damagetype)
		{
			case Attack::DamageType::DMG_WEAPON:
			case Attack::DamageType::DMG_MAGIC:
				mindamage = calculate_mindamage(leveldelta, attack.mindamage);
				maxdamage = calculate_maxdamage(leveldelta, attack.maxdamage);
				hitchance = calculate_hitchance(leveldelta, attack.accuracy);
				critical = attack.critical;
				break;
			case Attack::DamageType::DMG_FIXED:
				mindamage = attack.fixdamage;
				maxdamage = attack.fixdamage;
				hitchance = 1.0f;
				critical = 0.0f;
				break;
		}

		std::generate(
				result.begin(), result.end(),
				[&]()
				{
					return next_damage(mindamage, maxdamage, hitchance, critical, attack);
				}
		);

		return result;
	}

	std::pair<int32_t, bool> Mob::next_damage(double mindamage, double maxdamage, float hitchance, float critical,
											  const Attack &attack) const
	{
		bool hit = randomizer.below(hitchance);

		if (!hit)
			return std::pair<int32_t, bool>(0, false);

		constexpr double DAMAGECAP = 999999.0;

		double damage = randomizer.next_real(mindamage, maxdamage);
		bool iscritical = randomizer.below(critical);

		if (iscritical)
			damage *= 1.5;

		// A held skill hits harder the longer it was charged (0x79048c).
		damage = prepare::charge_percent(attack.skill, attack.charge) * damage * 0.01;

		if (damage < 1)
			damage = 1;
		else if (damage > DAMAGECAP)
			damage = DAMAGECAP;

		auto intdamage = static_cast<int32_t>(damage);

		return std::pair<int32_t, bool>(intdamage, iscritical);
	}

	int32_t Mob::next_magic_damage(const Attack &attack) const
	{
		using namespace magic_damage;

		double hitroll = randomizer.next_real(1.0);
		double accuracy = magic_damage::accuracy(attack.intelligence, attack.luck, attack.playerlevel, level);

		if (!hits(accuracy, avoid, hitroll))
			return 0;

		double roll = randomizer.next_real(1.0);
		double damage = attack.heal
				? heal::base(attack.heal, attack.magic, attack.intelligence, attack.luck, attack.healed, roll)
				: base(attack.magic, attack.intelligence, attack.mastery, attack.matk, roll);
		damage = damage * attack.amplify * 0.01;
		damage = elemental(damage, attack.skill, attack.element, elements);
		damage = defend(damage, mdef, randomizer.next_real(1.0));
		damage = prepare::charge_percent(attack.skill, attack.charge) * damage * 0.01;

		return finish(damage);
	}

	void Mob::apply_damage(int32_t damage, bool toleft)
	{
		hitsound.play();

		if (pending_hits > 0)
			pending_hits--;

		if (dying && stance != Stance::DIE)
		{
			if (pending_hits == 0)
				apply_death();
		} else if (control && is_alive() && damage >= knockback && !acting)
		{
			// It shows the hit at once; the AI flinches next update.
			show_stance(Stance::HIT, toleft);
			acting = true;
			flinch = Flinch{ toleft, std::nullopt };
		}
	}

	void Mob::catch_by(Point<int16_t> user)
	{
		if (!control || !is_alive() || acting || !canmove)
			return;

		// It turns to the user and flinches (0x66b6fc).
		show_stance(Stance::HIT, user.x() > get_position().x());
		acting = true;
		flinch = Flinch{ false, user };
	}

	MobAttack Mob::create_touch_attack() const
	{
		if (!deals_touch_damage())
			return MobAttack();

		// The base damage of the hit, a roll between 85% of the attack and all
		// of it, rounded down.
		int32_t attack = randomizer.next_int<int32_t>(touch_damage::min_base(watk), watk);

		return MobAttack(attack, get_position(), id, oid);
	}

	void Mob::apply_death()
	{
		show_stance(Stance::DIE, flip);
		diesound.play();
		dying = true;
	}

	bool Mob::is_alive() const
	{
		return active && !dying;
	}

	int32_t Mob::get_id() const
	{
		return id;
	}

	bool Mob::blocks_teleport() const
	{
		return blocksteleport;
	}

	bool Mob::is_undead() const
	{
		return undead;
	}

	bool Mob::is_boss() const
	{
		return boss;
	}

	bool Mob::can_move() const
	{
		return canmove;
	}

	bool Mob::deals_touch_damage() const
	{
		return touchdamage && !notattack;
	}

	bool Mob::is_in_range(const Rectangle<int16_t> &range) const
	{
		if (!active)
			return false;

		Rectangle<int16_t> bounds = animations.at(stance).get_bounds();
		bounds.shift(get_position());

		return range.overlaps(bounds);
	}

	Rectangle<int16_t> Mob::get_body() const
	{
		Rectangle<int16_t> body = animations.at(stance).get_bounds();

		if (flip && !noflip)
			body = Rectangle<int16_t>(-body.right(), -body.left(), body.top(), body.bottom());

		body.shift(get_position());

		return body;
	}

	Point<int16_t> Mob::get_head_position() const
	{
		Point<int16_t> position = get_position();

		return get_head_position(position);
	}
}