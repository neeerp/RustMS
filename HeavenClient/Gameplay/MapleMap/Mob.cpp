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

namespace ms
{
	Mob::Mob(int32_t oi, int32_t mid, int8_t mode, int8_t st, uint16_t fh, uint16_t homefh, bool newspawn, int8_t tm,
			 Point<int16_t> position, const Physics &physics) : MapObject(oi)
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
		knockback = info["pushed"];
		speed = info["speed"];
		flyspeed = info["flySpeed"];
		chasespeed = info["chaseSpeed"];
		touchdamage = info["bodyAttack"].get_bool();
		undead = info["undead"].get_bool();
		noflip = info["noFlip"].get_bool();
		notattack = info["notAttack"].get_bool();
		firstattack = info["firstAttack"].get_bool();
		showleveltag = !info["hideLevel"].get_bool() && !info["hideHP"].get_bool() &&
					   !info["hideName"].get_bool() && !info["boss"].get_bool() &&
					   (mid / 1000000 != 9 || mid / 100000 == 94 || mid / 100000 == 96);
		canjump = src["jump"].size() > 0;
		canfly = src["fly"].size() > 0;
		canmove = src["move"].size() > 0 || canfly;
		canchase = src["chase"].size() > 0;

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

		name = std::string(nl::nx::string["Mob.img"][std::to_string(mid)]["name"]);

		nl::node sndsrc = nl::nx::sound["Mob.img"][strid];

		hitsound = sndsrc["Damage"];
		diesound = sndsrc["Die"];

		// info/fs scales both the shoe's acceleration and its drag.
		phobj.shoe = true;
		phobj.walkforce = info["fs"].get_real(1.0);
		phobj.walkdrag = phobj.walkforce;
		update_shoe(false);

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
		set_control(mode);
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
		set_stance(st);
		haswaypoint = false;
		flygroup = -1;
		flycount = 0;
		flyx = 0;
		flyy = 0;
		flysteps = 0;
		pending_hits = 0;
		death_timeout = 0;

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

	void Mob::set_stance(uint8_t stancebyte)
	{
		flip = (stancebyte % 2) == 0;

		if (!flip)
			stancebyte -= 1;

		if (stancebyte < Stance::MOVE)
			stancebyte = Stance::MOVE;

		auto newstance = static_cast<Stance>(stancebyte);

		if (animations.count(newstance) == 0)
			newstance = Stance::MOVE;

		set_stance(newstance);
	}

	void Mob::set_stance(Stance newstance)
	{
		if (newstance != Stance::HIT)
			phobj.clear_flag(PhysicsObject::Flag::NOFRICTION);

		if (stance != newstance)
		{
			stance = newstance;

			animations.at(stance).reset();
		}
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
		showhp.update();

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
			if (!canfly)
			{
				if (phobj.is_flag_not_set(PhysicsObject::Flag::TURNATEDGES))
				{
					// A knocked back mob stops at the edge, still facing its
					// attacker.
					if (stance != Stance::HIT)
						flip = !flip;

					phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);
				}
			}

			if (canfly)
			{
				// Another client's flier glides the way it faces between the
				// controller's updates.
				if (!control)
				{
					flyx = stance == Stance::MOVE ? (flip ? 1 : -1) : 0;
					flyy = stance == Stance::HIT ? 0 : -1;
				}

				phobj.walkdir = flyx;
				phobj.flydir = flyy;
			}

			switch (stance)
			{
				case Stance::MOVE:
				case Stance::CHASE:
					if (!canfly)
						phobj.walkdir = flip ? 1 : -1;

					break;
				case Stance::HIT:
					if (phobj.is_flag_set(PhysicsObject::Flag::NOFRICTION) && phobj.onground)
					{
						// v83 slows the push by 40000 / mass (100) px/s^2
						// along the ground; walls and edges zero it.
						constexpr double KBDECEL = 400.0 * Constants::TIMESTEP * Constants::TIMESTEP / 1000000.0;

						double kbspeed = std::abs(phobj.hspeed) - KBDECEL;

						if (kbspeed > 0.0)
						{
							phobj.hspeed = phobj.hspeed < 0.0 ? -kbspeed : kbspeed;
						} else
						{
							phobj.hspeed = 0.0;
							phobj.clear_flag(PhysicsObject::Flag::NOFRICTION);
						}
					}

					break;
				case Stance::JUMP:
					// CVecCtrl::Jump (FUN_007f1bb0) speeds the mob up in the
					// direction it holds, and it keeps steering in the air.
					if (phobj.onground)
						Physics::jump(phobj, Physics::jump_speed(1.0), jumpdir);

					phobj.walkdir = jumpdir;

					break;
			}

			if (phobj.is_flag_set(PhysicsObject::Flag::JUMPER))
			{
				// At a loose end or a wall (FUN_007f8d64), a jumper jumps unless
				// it is chasing a target below it, and then only half the time.
				// Toward a target on its own group it walks down a step instead.
				const FootholdTree &fht = physics.get_fht();
				bool chasing = control && hastarget && stance != Stance::HIT;

				phobj.edgejump = !chasing || targetpos.y() <= phobj.get_y() || randomizer.next_bool();
				phobj.stepdown = !chasing || fht.get_group_below(targetpos) == fht.get_group(phobj.fhid);
			}

			physics.move_object(phobj);

			bool jumprequest = phobj.jumprequest;
			phobj.jumprequest = false;

			if (control)
			{
				// v83 gives up a chase 3 to 4 minutes after aggro (FUN_007f7f02).
				if (hastarget)
				{
					targettime -= Constants::TIMESTEP;

					if (targettime < 0)
						hastarget = false;
				}

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

				// v83 (FUN_005cc410) picks a walking mob's action as it goes:
				// its chase animation for as long as it has a target.
				if (!canfly && (stance == Stance::MOVE || stance == Stance::CHASE))
					walk(input());

				switch (stance)
				{
					case Stance::HIT:
						// v83 holds a hit mob still, then leaves it standing
						// until its AI next decides.
						if (aniend)
						{
							set_stance(Stance::STAND);
							update_movement();
						}

						break;
					case Stance::JUMP:
						// It lands still holding the direction it jumped in,
						// and its AI only counts down in the air.
						if (phobj.onground)
						{
							walk(jumpdir);
						} else
						{
							for (aitime += Constants::TIMESTEP; aitime >= 90; aitime -= 90)
								thinktimer--;
						}

						break;
					default:
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
				if (jumprequest && (stance == Stance::MOVE || stance == Stance::CHASE || stance == Stance::STAND))
					jump(input(), false);
			}
		} else
		{
			phobj.normalize();
			physics.get_fht().update_fh(phobj);

			if (death_timeout > 0 && --death_timeout == 0 && stance != Stance::DIE)
				apply_death();
		}

		return phobj.fhlayer;
	}

	void Mob::think_fly(const Physics &physics)
	{
		// FUN_007f8875 steers for the waypoint unless within 10 px of it
		// across, glides once within 10 px of its height, and beats up while
		// it is higher.
		if (haswaypoint)
		{
			double x = phobj.crnt_x();
			double y = phobj.crnt_y();
			int8_t xdir = 0;
			int8_t ydir = 0;

			if (std::abs(static_cast<int32_t>(x - waypoint.x())) > 9)
				xdir = x <= waypoint.x() ? 1 : -1;

			if (std::abs(static_cast<int32_t>(y - waypoint.y())) < 10)
				ydir = -1;
			else if (waypoint.y() < y)
				Physics::fly_jump(phobj);

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

		// It tells the server where it is at each new waypoint, and at
		// least once a second.
		if (!haswaypoint)
			pick_waypoint(physics);
		else if (++flysteps < 11)
			return;

		update_movement();
		flysteps = 0;
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
			set_stance(Stance::STAND);
		} else
		{
			set_stance(Stance::MOVE);
			flip = x > 0;
		}
	}

	void Mob::think(const Physics &physics)
	{
		if (phobj.is_flag_set(PhysicsObject::Flag::JUMPER) && think_jump(physics))
			return;

		// FUN_007f8025 (standing mobs), FUN_007f81c8 (walking) and
		// FUN_007f835e (jumping): each decision holds for a random time, in
		// 90 ms steps.
		if (thinktimer > 0)
		{
			thinktimer--;
			return;
		}

		Point<int16_t> position = get_position();
		int16_t dx = targetpos.x() - position.x();

		if (!canmove)
		{
			if (hastarget)
			{
				thinktimer = (randomizer.next_int(400) + 800) / 90;

				if (flip != (dx >= 0))
				{
					flip = dx >= 0;
					update_movement();
				}

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
		} else if (canjump && phobj.onground && randomizer.next_int(5) == 0)
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
		if (!phobj.onground)
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

			if (dir != 0 && phobj.get_y() == jumpy)
			{
				dir = -dir;
				walk(dir);
			}
		}

		// It jumps at a rising wall up to 40 px ahead while its decision
		// lasts, or turns back once pressed against it.
		const FootholdTree &fht = physics.get_fht();

		switch (fht.get_wall_ahead(phobj, dir))
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
			int16_t x = phobj.get_x();

			if (group != fht.get_group(phobj.fhid) && targetpos.y() < phobj.get_y() &&
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
		jumpy = phobj.get_y();
		set_stance(Stance::JUMP);
		update_movement();
	}

	int8_t Mob::input() const
	{
		switch (stance)
		{
			case Stance::MOVE:
			case Stance::CHASE:
				return flip ? 1 : -1;
			case Stance::JUMP:
				return jumpdir;
			default:
				return 0;
		}
	}

	void Mob::walk(int8_t direction)
	{
		Stance before = stance;
		bool faced = flip;

		if (direction == 0)
		{
			set_stance(Stance::STAND);
		} else
		{
			set_stance(hastarget && canchase ? Stance::CHASE : Stance::MOVE);
			flip = direction > 0;
		}

		if (stance != before || flip != faced)
			update_movement();
	}

	void Mob::update_movement()
	{
		MoveMobPacket(
				oid, 1, 0, 0, 0, 0, 0, 0, get_position(),
				Movement(phobj, value_of(stance, flip))
		).dispatch();
	}

	void Mob::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		effects.drawbelow(absp, alpha);

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

		effects.drawabove(absp, alpha);
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
		control = mode > 0;

		// v83 (FUN_005d511d) has a mob given with aggro chase the local
		// player.
		if (mode == 2)
		{
			chase();
		} else if (mode == 0)
		{
			hastarget = false;
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
		auto shoespeed = [](float base)
		{
			return std::clamp(base + 100.0f, 10.0f, 140.0f) * 0.01;
		};

		float base = canfly ? flyspeed : speed;

		phobj.walkspeed = canmove ? shoespeed(chasing && canchase ? chasespeed : base) : 0.0;

		if (canfly)
			phobj.flyspeed = shoespeed(base);
	}

	void Mob::set_target(Point<int16_t> position, bool dead)
	{
		targetpos = position;
		targetdead = dead;
	}

	void Mob::send_movement(Point<int16_t> start, std::vector<Movement> &&in_movements)
	{
		if (control)
			return;

		set_position(start);

		movements = std::forward<decltype(in_movements)>(in_movements);

		if (movements.empty())
			return;

		const Movement &lastmove = movements.front();

		uint8_t laststance = lastmove.newstate;
		set_stance(laststance);
		update_shoe(stance == Stance::CHASE);
		jumpdir = 0;

		phobj.fhid = lastmove.fh;
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

	void Mob::show_effect(const Animation &animation, int8_t pos, int8_t z, bool f)
	{
		if (!active)
			return;

		Point<int16_t> shift;

		switch (pos)
		{
			case 0:
				shift = get_head_position(Point<int16_t>());
				break;
			case 1:
				break;
			case 2:
				break;
			case 3:
				break;
			case 4:
				break;
		}

		effects.add(animation, DrawArgument(shift, f), z);
	}

	void Mob::show_hit(const Animation &animation, const Rectangle<int16_t> &range, bool f)
	{
		if (!active)
			return;

		Rectangle<int16_t> body = animations.at(stance).get_bounds();
		body.shift(get_position());

		// As v83 does, centre the spark on the overlap of the attack and the
		// body, first stretching an attack that falls short to touch it.
		int16_t left = range.left();
		int16_t right = range.right();
		int16_t top = range.top();
		int16_t bottom = range.bottom();

		if (left >= body.right())
			left = body.right() - 1;
		else if (right <= body.left())
			right = body.left() + 1;

		if (top >= body.bottom())
			top = body.bottom() - 1;
		else if (bottom <= body.top())
			bottom = body.top() + 1;

		int16_t x = (std::max(left, body.left()) + std::min(right, body.right())) / 2;
		int16_t y = (std::max(top, body.top()) + std::min(bottom, body.bottom())) / 2;

		effects.add(animation, DrawArgument(Point<int16_t>(x, y) - get_position(), f), 0);
	}

	float Mob::calculate_hitchance(int16_t leveldelta, int32_t player_accuracy) const
	{
		float faccuracy = static_cast<float>(player_accuracy);
		float hitchance = faccuracy / (((1.84f + 0.07f * leveldelta) * avoid) + 1.0f);

		if (hitchance < 0.01f)
			hitchance = 0.01f;

		return hitchance;
	}

	double Mob::calculate_mindamage(int16_t leveldelta, double damage, bool magic) const
	{
		double mindamage =
				magic ?
				damage - (1 + 0.01 * leveldelta) * mdef * 0.6 :
				damage * (1 - 0.01 * leveldelta) - wdef * 0.6;

		return mindamage < 1.0 ? 1.0 : mindamage;
	}

	double Mob::calculate_maxdamage(int16_t leveldelta, double damage, bool magic) const
	{
		double maxdamage =
				magic ?
				damage - (1 + 0.01 * leveldelta) * mdef * 0.5 :
				damage * (1 - 0.01 * leveldelta) - wdef * 0.5;

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

		switch (damagetype)
		{
			case Attack::DamageType::DMG_WEAPON:
			case Attack::DamageType::DMG_MAGIC:
				mindamage = calculate_mindamage(leveldelta, attack.mindamage,
												damagetype == Attack::DamageType::DMG_MAGIC);
				maxdamage = calculate_maxdamage(leveldelta, attack.maxdamage,
												damagetype == Attack::DamageType::DMG_MAGIC);
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

		std::vector<std::pair<int32_t, bool>> result(attack.hitcount);

		std::generate(
				result.begin(), result.end(),
				[&]()
				{
					return next_damage(mindamage, maxdamage, hitchance, critical);
				}
		);

		update_movement();

		return result;
	}

	std::pair<int32_t, bool> Mob::next_damage(double mindamage, double maxdamage, float hitchance, float critical) const
	{
		bool hit = randomizer.below(hitchance);

		if (!hit)
			return std::pair<int32_t, bool>(0, false);

		constexpr double DAMAGECAP = 999999.0;

		double damage = randomizer.next_real(mindamage, maxdamage);
		bool iscritical = randomizer.below(critical);

		if (iscritical)
			damage *= 1.5;

		if (damage < 1)
			damage = 1;
		else if (damage > DAMAGECAP)
			damage = DAMAGECAP;

		auto intdamage = static_cast<int32_t>(damage);

		return std::pair<int32_t, bool>(intdamage, iscritical);
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
		} else if (control && is_alive() && damage >= knockback && stance != Stance::HIT)
		{
			flip = toleft;
			set_stance(Stance::HIT);

			// It stops steering while hit, so a flier dives (FUN_007f7f02).
			flyx = 0;
			flyy = 0;

			// v83 pushes a flinching mob away from its attacker at 130 px/s.
			if (canmove)
			{
				constexpr double KBSPEED = 130.0 * Constants::TIMESTEP / 1000.0;

				phobj.hspeed = flip ? -KBSPEED : KBSPEED;
				phobj.set_flag(PhysicsObject::Flag::NOFRICTION);
			}

			update_movement();
		}
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
		set_stance(Stance::DIE);
		diesound.play();
		dying = true;
	}

	bool Mob::is_alive() const
	{
		return active && !dying;
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

	Point<int16_t> Mob::get_head_position() const
	{
		Point<int16_t> position = get_position();

		return get_head_position(position);
	}
}