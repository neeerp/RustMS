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

#include "../Combat/Assassinate.h"
#include "../Combat/Heal.h"
#include "../Combat/HitPoint.h"
#include "../Combat/MesoExplosion.h"
#include "../Combat/MonsterMagnet.h"
#include "../Combat/Prepare.h"
#include "../Combat/ShotPath.h"
#include "../../Character/SkillId.h"
#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../IO/UI.h"
#include "../../IO/UITypes/UIMobGage.h"

#include "../../Net/Packets/GameplayPackets.h"
#include "../Stage.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <climits>
#include <numeric>
#include <tuple>
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
		int32_t destruct = info["selfDestruction"]["action"];
		selfdestructs = destruct != 0;
		bombs = selfdestructs && (destruct & 2) != 0;
		bombed = false;
		showleveltag = !info["hideLevel"].get_bool() && !info["hideHP"].get_bool() &&
					   !info["hideName"].get_bool() && !info["boss"].get_bool() &&
					   (mid / 1000000 != 9 || mid / 100000 == 94 || mid / 100000 == 96);
		canjump = src["jump"].size() > 0;
		canfly = src["fly"].size() > 0;
		canmove = src["move"].size() > 0 || canfly;
		canchase = src["chase"].size() > 0;
		boss = info["boss"].get_bool();
		gaugehide = info["HPgaugeHide"].get_bool();
		tagcolor = static_cast<uint8_t>(info["hpTagColor"].get_integer());
		tagbgcolor = static_cast<uint8_t>(info["hpTagBgcolor"].get_integer());
		hitbyplayer = false;
		fixeddamage = info["fixedDamage"];
		maxhp = info["maxHP"];
		invincible = info["invincible"].get_bool();

		for (nl::node skill : info["damagedBySelectedSkill"])
			damagedby.push_back(skill.get_integer());

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

		mp = info["maxMP"];

		// attack1, attack2 and on, as long as each has its info
		// (FUN_0067fd81).
		for (int32_t n = 1;; n++)
		{
			nl::node attacksrc = src["attack" + std::to_string(n)];
			nl::node attackinfo = attacksrc["info"];

			if (attackinfo.size() == 0)
				break;

			nl::node range = attackinfo["range"];
			Point<int16_t> lt = range["lt"];
			Point<int16_t> rb = range["rb"];
			mob_attack::Box box{ lt.x(), lt.y(), rb.x(), rb.y() };

			AttackInfo attack;
			attack.type = attackinfo["type"];
			attack.magic = attackinfo["magic"].get_bool();
			attack.element = 0;

			// Only a magic attack has an element (FUN_006811a7).
			if (attack.magic)
			{
				std::string element = attackinfo["elemAttr"];

				switch (element.empty() ? 0 : std::tolower(static_cast<unsigned char>(element[0])))
				{
					case 'i':
						attack.element = 1;
						break;
					case 'f':
						attack.element = 2;
						break;
					case 'l':
						attack.element = 3;
						break;
					case 's':
						attack.element = 4;
						break;
				}
			}

			attack.watk = attackinfo["PADamage"].get_integer(watk);
			attack.conmp = attackinfo["conMP"];
			attack.after = attackinfo["attackAfter"];
			attack.effectafter = attackinfo["effectAfter"];
			attack.jump = attackinfo["jumpAttack"].get_bool();
			attack.tremble = attackinfo["tremble"].get_bool();
			attack.deadly = attackinfo["deadlyAttack"].get_bool();
			attack.mpburn = attackinfo["mpBurn"].get_bool();
			attack.range = box;
			attack.sp = range["sp"];
			attack.r = range["r"];
			attack.area = { box, range["start"], range["areaCount"] };
			attack.attackcount = range["attackCount"];
			attack.bulletspeed = attackinfo["bulletSpeed"];
			attack.animation = attacksrc;
			attack.hit = attackinfo["hit"];

			if (attackinfo["effect"].size() > 0)
				attack.effect = Animation(attackinfo["effect"]);

			if (attackinfo["ball"].size() > 0)
				attack.ball = Animation(attackinfo["ball"]);

			if (attackinfo["areaWarning"].size() > 0)
				attack.warning = Animation(attackinfo["areaWarning"]);

			attacks.push_back(std::move(attack));
		}

		// Its skills, and the skill<n> animations they play.
		for (nl::node skillsrc : info["skill"])
		{
			SkillInfo skill;
			skill.skill = static_cast<uint8_t>(skillsrc["skill"].get_integer());
			skill.level = static_cast<uint8_t>(skillsrc["level"].get_integer());
			skill.action = static_cast<uint8_t>(skillsrc["action"].get_integer());
			skill.effectafter = skillsrc["effectAfter"];

			if (skill.skill == 0 || skill.action == 0)
				continue;

			if (skillanimations.count(skill.action) == 0)
				skillanimations[skill.action] = src["skill" + std::to_string(skill.action)];

			skills.push_back(skill);
		}

		// Leave type 3 plays skill1 whether it casts or not.
		if (skillanimations.count(1) == 0 && src["skill1"].size() > 0)
			skillanimations[1] = src["skill1"];

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
		leaving = false;
		exploding = false;
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

		if (fadein)
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

		clock += Constants::TIMESTEP;

		// It blows up with attack1, which hits like any other (FUN_00663e5b).
		if (exploding)
		{
			exploding = false;
			casting.reset();

			if (!attacks.empty())
				do_attack(physics, 0, noflip || !flip, 0, get_position());
		}

		if (attacking && attacks[*attacking].animation.update())
			attacking.reset();

		if (casting && skillanimations.at(*casting).update())
			casting.reset();

		update_attacks();

		if (leaving && !attacking && !casting && pendinghits.empty())
		{
			dead = true;

			return phobj.fhlayer;
		}

		for (StatusEffect &effect : statuseffects)
			effect.animation.update();
		showhp.update();
		keepsgage.update();

		if (gaugehide && hitbyplayer && !keepsgage)
			if (auto gage = UI::get().get_element<UIMobGage>())
				gage->hide();

		if (!dying)
		{
			for (int32_t number : hurts.update(statuses, fixeddamage, Constants::TIMESTEP, damagedby))
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

		// Every client tells the server once the player is in reach of a
		// bombing mob's plain attack, the box as it is in the data
		// (FUN_0066e636).
		if (bombs && !bombed && !dying && !notattack && !held() && !targetdead)
		{
			Point<int16_t> position = get_position();

			for (const AttackInfo &info : attacks)
			{
				if (info.type == 0 && mob_attack::intersects(info.range.shift(position.x(), position.y()), targetbody))
				{
					bombed = true;
					MobBombPacket(oid).dispatch();
					break;
				}
			}
		}

		if (!dying)
		{
			// Another client's mob attacks where its path starts (FUN_0066be61).
			if (remoteattack)
			{
				do_attack(physics, remoteattack->first.index, remoteattack->first.left,
						  remoteattack->first.skilldata, remoteattack->second);
				remoteattack.reset();
			}

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

				// Once a move is acked with leave to attack, it tries every
				// update for an attack that reaches (FUN_006675a8).
				// A skill it was let cast goes first (FUN_0066e7af); a cast
				// sends its own path.
				std::optional<Pick> pick;

				if (!flinch && ctrlstate == 4 && !cast(physics))
					pick = pick_attack(physics);

				if (flinch)
				{
					flinch_body(physics);
				} else if (pick)
				{
					attack(physics, *pick);
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
		attackhold = 0;

		recorder.reset(Movement(Movement::ABSOLUTE, 0, body.get_x(), body.get_y(), Movement::velocity(body.hspeed),
								Movement::velocity(body.vspeed), end.fh, value_of(bodystance, bodyflip), 0));
		seated = true;
	}

	void Mob::plan(const Physics &physics, int8_t action, bool left, int32_t skilldata)
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

		MoveMobPacket(oid, ++moveid, flags, action, left, skilldata, canfly, path).dispatch();

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

	std::optional<Mob::Pick> Mob::pick_attack(const Physics &physics)
	{
		// It attacks the player it chases, not a Puppet, from the ground or
		// in the air, while not hit, attacking or held (FUN_006675a8).
		if (attacks.empty() || acting || attacking || casting || !hastarget || puppet || targetdead || held())
			return std::nullopt;

		if (!phobj.onground && !canfly)
			return std::nullopt;

		Point<int16_t> position = get_position();
		int32_t x = position.x();
		int32_t y = position.y();
		bool facingleft = noflip || !flip;
		std::vector<Pick> candidates;

		// One that destroys itself keeps its last attack for that
		// (FUN_0066a517).
		size_t usable = attacks.size() - (selfdestructs ? 1 : 0);

		for (size_t i = 0; i < usable; i++)
		{
			const AttackInfo &info = attacks[i];

			// Dark Sight keeps physical attacks off.
			if (mp < info.conmp || (!info.magic && targetdarksight))
				continue;

			std::optional<bool> left;
			int32_t skilldata = 0;

			switch (info.type)
			{
				case 0:
				{
					bool leftward = mob_attack::intersects(info.range.shift(x, y), targetbody);
					bool rightward = mob_attack::intersects(info.range.mirror().shift(x, y), targetbody);

					left = mob_attack::choose_facing(facingleft, !noflip, leftward, rightward);

					break;
				}
				case 1:
				case 2:
				{
					// A cone from `sp` out to the range r (FUN_0066afb7).
					int32_t oy = y + info.sp.y();
					int32_t lx = mob_attack::origin_x(x, info.sp.x(), true);
					int32_t rx = mob_attack::origin_x(x, info.sp.x(), false);
					bool leftward = mob_attack::cone_reaches(lx, lx - info.r, oy, targetbody);
					bool rightward = mob_attack::cone_reaches(rx, rx + info.r, oy, targetbody);

					left = mob_attack::choose_facing(facingleft, !noflip, leftward, rightward);

					if (!left)
						break;

					// A shot names its target; a ball aims at a point.
					if (info.type == 1)
					{
						skilldata = targetid << 2;
					} else
					{
						auto [ax, ay] = mob_attack::aim(targetbody, oy, *left);

						skilldata = (ax & 0xFFFF) | (ay << 16);
					}

					break;
				}
				case 3:
				case 4:
				{
					// At most one area attack in 2.5 s, only the way it
					// faces, on attackCount cells at random; ground cells
					// need ground under them (FUN_0066aebf).
					if (clock - lastarea <= 2499)
						break;

					if (!mob_attack::intersects(mob_attack::area_box(info.area, x, y, facingleft), targetbody))
						break;

					std::vector<int32_t> cells(std::max(info.area.count, 0));
					std::iota(cells.begin(), cells.end(), 0);

					for (size_t c = cells.size(); c > 1; c--)
						std::swap(cells[c - 1], cells[randomizer.next_int(c)]);

					cells.resize(std::min<size_t>(cells.size(), std::max(info.attackcount, 0)));

					for (int32_t cell : cells)
					{
						int32_t cx = mob_attack::cell_x(info.area, x, cell, facingleft);

						if (info.type == 3 && !ground_below(physics, static_cast<int16_t>(cx), position.y()))
							continue;

						if (cell < 32)
							skilldata |= 1 << cell;
					}

					if (skilldata != 0)
						left = facingleft;

					break;
				}
			}

			if (left)
				candidates.push_back({ static_cast<uint8_t>(i), *left, skilldata });
		}

		if (candidates.empty())
			return std::nullopt;

		return candidates[randomizer.next_int(candidates.size())];
	}

	void Mob::attack(const Physics &physics, const Pick &pick)
	{
		do_attack(physics, pick.index, pick.left, pick.skilldata, get_position());

		// It stops what it shows, turns, and stands for its attack before
		// the AI goes on (FUN_0066b6fc).
		shown.land(canfly || !canmove);
		seat_body(physics);

		bodyflip = !pick.left;
		set_body_stance(Stance::STAND);
		attackhold = attacks[pick.index].animation.get_duration();
		flyx = 0;
		flyy = 0;

		plan(physics, static_cast<int8_t>(ATTACK1 + pick.index), pick.left, pick.skilldata);
	}

	void Mob::do_attack(const Physics &physics, uint8_t index, bool left, int32_t skilldata, Point<int16_t> at)
	{
		if (held())
			return;

		AttackInfo &info = attacks[index];
		int32_t x = at.x();
		int32_t y = at.y();

		attacking = index;
		attackleft = left;
		info.animation.reset();

		if (info.effect)
			pendingeffects.push_back({ clock + std::max(info.effectafter, 1), *info.effect, left });

		int32_t due = clock + info.after;

		// v83 shakes the view as it lands, whoever it hits (0x6671fa).
		if (info.tremble)
			trembleat = due;

		switch (info.type)
		{
			case 0:
				// The box is where the mob attacked, wherever it goes after.
				pendinghits.push_back({ due, index, left, (left ? info.range : info.range.mirror()).shift(x, y) });
				break;
			case 1:
			{
				// A shot at the local player hits once it lands, if they
				// were in reach when it went.
				if ((skilldata & 3) != 0 || (skilldata >> 2) != targetid)
					break;

				int32_t ox = mob_attack::origin_x(x, info.sp.x(), left);
				int32_t oy = y + info.sp.y();

				if (mob_attack::shot_reaches(targetbody, ox, oy, info.r, left))
					pendinghits.push_back({ due, index, left, std::nullopt });

				break;
			}
			case 2:
			{
				if (!info.ball)
					break;

				int32_t ox = mob_attack::origin_x(x, info.sp.x(), left);
				int32_t oy = y + info.sp.y();
				int16_t tx = static_cast<int16_t>(skilldata & 0xFFFF);
				int16_t ty = static_cast<int16_t>((skilldata >> 16) & 0xFFFF);
				auto [ex, ey] = mob_attack::ball_end(ox, oy, tx, ty, info.r, left);
				int32_t flight = mob_attack::ball_time(info.r, info.bulletspeed);

				balls.push_back({ due, due + std::max(flight, 1),
								  Point<int16_t>(static_cast<int16_t>(ox), static_cast<int16_t>(oy)),
								  Point<int16_t>(static_cast<int16_t>(ex), static_cast<int16_t>(ey)),
								  index, left, *info.ball });

				break;
			}
			case 3:
			case 4:
			{
				int32_t width = info.area.width();
				int32_t count = 0;

				for (int32_t cell = 0; cell < 32 && count < info.attackcount; cell++)
				{
					if (((skilldata >> cell) & 1) == 0)
						continue;

					count++;

					int32_t cx = mob_attack::cell_x(info.area, x, cell, left);
					int32_t cy = y;

					if (info.type == 3)
					{
						std::optional<int16_t> ground = ground_below(physics, static_cast<int16_t>(cx), at.y());

						if (!ground)
							continue;

						cy = *ground;
					}

					mob_attack::Box box = mob_attack::cell_box(info.area, cx, cy, left);
					pendinghits.push_back({ due, index, left, box });

					if (info.warning)
					{
						Point<int16_t> warning(static_cast<int16_t>(box.left + width / 2),
											   static_cast<int16_t>(box.bottom));

						fieldeffects.add(*info.warning, DrawArgument(warning), 0);
					}
				}

				lastarea = clock;

				break;
			}
		}
	}

	bool Mob::cast(const Physics &physics)
	{
		if (pendingskill == 0 || acting || attacking || casting || held())
			return false;

		auto info = std::find_if(skills.begin(), skills.end(),
								 [this](const SkillInfo &skill) { return skill.skill == pendingskill; });

		// One it doesn't have stays pending.
		if (info == skills.end())
			return false;

		bool left = noflip || !flip;
		uint8_t skill = pendingskill;
		uint8_t skilllevel = pendingskilllevel;

		if (!do_cast(skill, skilllevel, left))
			return false;

		pendingskill = 0;
		pendingskilllevel = 0;

		// Like an attack, it stands still while the skill plays.
		shown.land(canfly || !canmove);
		seat_body(physics);

		bodyflip = !left;
		set_body_stance(Stance::STAND);

		int32_t duration = skillanimations.at(info->action).get_duration();
		attackhold = static_cast<int16_t>(std::min<int32_t>(duration, INT16_MAX));
		flyx = 0;
		flyy = 0;

		plan(physics, mob_attack::skill_action(info->action), left,
			 mob_attack::skill_data(skill, skilllevel, duration));

		return true;
	}

	bool Mob::do_cast(uint8_t skill, uint8_t skilllevel, bool left)
	{
		auto info = std::find_if(skills.begin(), skills.end(),
								 [skill](const SkillInfo &other) { return other.skill == skill; });

		if (info == skills.end())
			return false;

		nl::node level = nl::nx::skill["MobSkill.img"][std::to_string(skill)]["level"][std::to_string(skilllevel)];

		if (!level)
			return false;

		Animation &animation = skillanimations.at(info->action);
		animation.reset();
		casting = info->action;
		attackleft = left;

		nl::node effect = level["effect"];

		if (effect.size() > 0)
			pendingeffects.push_back({ clock + std::max(info->effectafter, 1), effect, left,
									   static_cast<int8_t>(effect["pos"].get_integer()) });

		return true;
	}

	void Mob::show_summon(int8_t type, int32_t delay)
	{
		nl::node effect = nl::nx::effect["Summon.img"][std::to_string(type)];

		if (effect.size() > 0)
			pendingeffects.push_back({ clock + std::max(delay, 1), effect, noflip || !flip });
	}

	void Mob::update_attacks()
	{
		for (auto iter = pendingeffects.begin(); iter != pendingeffects.end();)
		{
			if (clock < iter->due)
			{
				++iter;
				continue;
			}

			if (iter->pos)
				show_effect(iter->animation, *iter->pos, 0, !iter->left, get_body());
			else
				effects.add(iter->animation, DrawArgument(Point<int16_t>(0, 0), !iter->left), 0);

			iter = pendingeffects.erase(iter);
		}

		if (trembleat && clock >= *trembleat)
		{
			trembleat.reset();
			Stage::get().tremble(30, true, 0, 0, false);
		}

		// A box hits the player where they are once due; a jump attack only
		// one standing on a foothold. A direct shot hits regardless.
		for (auto iter = pendinghits.begin(); iter != pendinghits.end();)
		{
			if (clock < iter->due)
			{
				++iter;
				continue;
			}

			const AttackInfo &info = attacks[iter->index];
			bool hit = !iter->box || (mob_attack::intersects(*iter->box, targetbody) &&
									  (info.type != 0 || !info.jump || targetonground));

			if (hit && !targetdead)
				landed.push_back({ make_attack(iter->index, iter->left), attacks[iter->index].hit });

			iter = pendinghits.erase(iter);
		}

		// A ball hits the player once its point is in them; the player's
		// invincibility takes the rest.
		for (auto iter = balls.begin(); iter != balls.end();)
		{
			if (clock >= iter->end)
			{
				iter = balls.erase(iter);
				continue;
			}

			if (clock >= iter->start)
			{
				iter->animation.update();

				double progress = static_cast<double>(clock - iter->start) / (iter->end - iter->start);
				int32_t bx = static_cast<int32_t>(iter->from.x() + (iter->to.x() - iter->from.x()) * progress);
				int32_t by = static_cast<int32_t>(iter->from.y() + (iter->to.y() - iter->from.y()) * progress);

				if (mob_attack::contains(targetbody, bx, by) && !targetdead)
					landed.push_back({ make_attack(iter->index, iter->left), attacks[iter->index].hit });
			}

			++iter;
		}
	}

	MobAttack Mob::make_attack(uint8_t index, bool left) const
	{
		const AttackInfo &info = attacks[index];

		MobAttack attack = create_touch_attack_base();
		attack.watk = info.watk + status_value(MobStatParser::PAD);
		attack.index = static_cast<int8_t>(index);
		attack.magic = info.magic;
		attack.element = info.element;
		attack.deadly = info.deadly;
		attack.mpburn = info.mpburn;
		attack.pushleft = left;

		return attack;
	}

	std::optional<int16_t> Mob::ground_below(const Physics &physics, int16_t x, int16_t y) const
	{
		// The highest ground from 100 px above to 100 px below (FUN_00a456f9).
		const FootholdTree &fht = physics.get_fht();
		uint16_t fhid = fht.get_fhid_below(Point<int16_t>(x, static_cast<int16_t>(y - 100)));

		if (fhid == 0)
			return std::nullopt;

		double ground = fht.get_fh(fhid).ground_below(x);

		if (ground > y + 100.0)
			return std::nullopt;

		return static_cast<int16_t>(ground);
	}

	void Mob::step_body(const Physics &physics)
	{
		// A stunned, frozen or webbed mob stands where it is, and its AI
		// waits (inferred: the hold itself wasn't traced in v83).
		// It stands still for its attack too (FUN_009bbdfd).
		bool hold = (held() || attackhold > 0) && bodystance != Stance::HIT && bodystance != Stance::JUMP;

		if (attackhold > 0)
			attackhold -= Constants::TIMESTEP;

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

	void Mob::acknowledge(int16_t ackid, bool nextattack, int16_t newmp, uint8_t skill, uint8_t skilllevel)
	{
		if (!control)
			set_control(1);

		mp = newmp;
		std::tie(pendingskill, pendingskilllevel) =
				mob_attack::granted_skill({ pendingskill, pendingskilllevel }, { skill, skilllevel });

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

			if (attacking)
				attacks[*attacking].animation.draw(DrawArgument(absp, !attackleft, interopc), alpha);
			else if (casting)
				skillanimations.at(*casting).draw(DrawArgument(absp, !attackleft, interopc), alpha);
			else
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

		// A ball turns to its flight, drawn for a mob facing left.
		for (const Ball &ball : balls)
		{
			if (clock < ball.start)
				continue;

			double progress = static_cast<double>(clock - ball.start) / (ball.end - ball.start);
			double dx = ball.to.x() - ball.from.x();
			double dy = ball.to.y() - ball.from.y();
			Point<int16_t> at(static_cast<int16_t>(ball.from.x() + dx * progress),
							  static_cast<int16_t>(ball.from.y() + dy * progress));
			float angle = static_cast<float>(ball.left ? std::atan2(-dy, -dx) : std::atan2(dy, dx));

			ball.animation.draw(DrawArgument(angle, viewp + at, !ball.left, 1.0f), alpha);
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

	void Mob::set_target(const Target &target)
	{
		targetpos = puppet ? puppetpos : target.position;
		targetdead = puppet ? false : target.dead;
		targetid = target.id;
		targetbody = target.body;
		targetonground = target.onground;
		targetdarksight = target.darksight;
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

	void Mob::send_movement(Point<int16_t> start, std::vector<Movement> &&in_movements, int8_t action, bool left,
							int32_t skilldata)
	{
		// v83 plays the path on from where the mob is shown; its start is
		// not used (FUN_0068b371), except that an attack is made there.
		if (control)
			return;

		shown.receive(in_movements);

		int32_t index = action - ATTACK1;

		if (index >= 0 && index < static_cast<int32_t>(attacks.size()))
			remoteattack = std::make_pair(Pick{ static_cast<uint8_t>(index), left, skilldata }, start);

		// Another client's mob casts a skill it has (FUN_0066be61).
		if (action >= mob_attack::skill_action(1))
		{
			auto [skill, skilllevel] = mob_attack::cast_skill(skilldata);
			do_cast(skill, skilllevel, left);
		}
	}

	std::vector<Mob::Landed> Mob::take_attacks()
	{
		return std::exchange(landed, {});
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
			case 3:
				// skill1 in place of its death; one without just goes
				// (FUN_00663995).
				dying = true;
				leaving = true;
				attacking.reset();
				casting.reset();
				diesound.play();
				tremble_on_death();

				if (skillanimations.count(1) > 0)
				{
					skillanimations.at(1).reset();
					casting = 1;
					attackleft = noflip || !flip;
				}

				break;
			default:
				// It blows up (FUN_00663e5b).
				dying = true;
				leaving = true;
				exploding = true;
				attacking.reset();
				diesound.play();
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
		// A boss shows its HP only in the bar the server sends (0x66d652).
		if (boss && !gaugehide)
			return;

		// A mob that hides its gauge shows the boss bar instead, with v83's 32-bit arithmetic (0x66880a).
		if (gaugehide)
		{
			hitbyplayer = true;
			keepsgage.set_for(20000);
			auto hp = static_cast<int32_t>(static_cast<uint32_t>(maxhp) * static_cast<uint8_t>(percent));
			UIMobGage::show(id, hp / 100, maxhp, tagcolor, tagbgcolor);

			return;
		}

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

	void Mob::show_damage(int32_t damage)
	{
		if (active)
			hurtnumbers.push_back(damage);
	}

	void Mob::show_reflect(uint8_t percent, int32_t damage, bool physical)
	{
		int32_t reflected = percent * damage / 100;

		if (physical)
			reflected = std::min(reflected, maxhp / 10);

		show_damage(reflected);
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

	// A monster's PDR status adds to its weapon defense.
	int32_t Mob::defense() const
	{
		return wdef + status_value(MobStatParser::PDR);
	}

	bool Mob::blow_hits(const Attack &attack) const
	{
		using namespace physical_damage;

		// A monster's EVA status adds to its avoid (FUN_0078df87).
		double accuracy = physical_damage::accuracy(attack.accuracy, attack.playerlevel, level);

		if (!hits(accuracy, avoid + status_value(MobStatParser::EVA), randomizer.next_real(1.0)))
			return false;

		return !attack.blow.blind || sees(randomizer.next_real(1.0));
	}

	std::vector<std::pair<int32_t, bool>> Mob::calculate_damage(const Attack &attack)
	{
		std::vector<std::pair<int32_t, bool>> result(attack.hitcount);

		// Any other attack misses a mob that only some skills hurt, as
		// Kyrin's test Octopirates.
		if (!MobHurts::hurt_by(damagedby, attack.skill))
			return result;

		for (size_t i = 0; i < result.size(); i++)
		{
			switch (attack.damagetype)
			{
				// A spell rolls by v83's own formula, and never crits.
				case Attack::DamageType::DMG_MAGIC:
					result[i] = { next_magic_damage(attack), false };
					break;
				case Attack::DamageType::DMG_WEAPON:
					result[i] = next_blow(attack, result, i);
					break;
				default:
					result[i] = next_damage(attack);
					break;
			}
		}

		return result;
	}

	std::vector<int32_t> Mob::explode(int32_t x, const std::vector<int32_t> &mesos) const
	{
		std::vector<int32_t> lines;

		for (int32_t amount : mesos)
		{
			if (status_value(MobStatParser::DISABLE))
				lines.push_back(0);
			else if (status_value(MobStatParser::PIMMUNE) && !magic_damage::pierces(randomizer.next_int(100), 0))
				lines.push_back(1);
			else
				lines.push_back(meso_explosion::line(x, amount, randomizer.next_real(0.5, 1.0)));
		}

		return lines;
	}

	int32_t Mob::shown_explosion(int32_t number) const
	{
		return meso_explosion::shown(number, fixeddamage, MobHurts::hurt_by(damagedby, meso_explosion::ID));
	}

	std::pair<int32_t, bool> Mob::next_damage(const Attack &attack) const
	{
		using namespace magic_damage;

		constexpr double DAMAGECAP = 999999.0;

		bool meso = attack.damagetype == Attack::DamageType::DMG_MESO;
		double damage = attack.fixdamage;
		bool iscritical = false;

		// Shadow Meso meets the statuses and misses as a blow does
		// (FUN_0078df87), but fixed damage skips them.
		if (meso)
		{
			if (status_value(MobStatParser::DISABLE))
				return std::pair<int32_t, bool>(0, false);

			if (status_value(MobStatParser::PIMMUNE) && !pierces(randomizer.next_int(100), 0))
				return std::pair<int32_t, bool>(1, false);

			if (!blow_hits(attack))
				return std::pair<int32_t, bool>(0, false);

			iscritical = randomizer.below(attack.critical);
			damage = shot_path::meso_damage(randomizer.next_real(attack.mindamage, attack.maxdamage), iscritical,
				attack.mesocritical, static_cast<int16_t>(std::max(level - attack.playerlevel, 0)));
			damage = guarded(damage, status_value(MobStatParser::PGUARD_UP));
		}

		// A held skill hits harder the longer it was charged (0x79048c).
		damage = prepare::charge_percent(attack.skill, attack.charge) * damage * 0.01;

		// Only a critical hit gets through Hard Skin (0x7904ba).
		if (meso && status_value(MobStatParser::HARD_SKIN) && !iscritical)
			return std::pair<int32_t, bool>(0, false);

		if (damage < 1)
			damage = 1;
		else if (damage > DAMAGECAP)
			damage = DAMAGECAP;

		return std::pair<int32_t, bool>(static_cast<int32_t>(damage), iscritical);
	}

	std::pair<int32_t, bool> Mob::next_blow(const Attack &attack, const std::vector<std::pair<int32_t, bool>> &hits,
											size_t index) const
	{
		using namespace physical_damage;

		const Blow &blow = attack.blow;

		// Disable turns every blow away, and Physical Immunity all but 1 in
		// 100 to 1 damage (FUN_0078df87).
		if (status_value(MobStatParser::DISABLE))
			return std::pair<int32_t, bool>(0, false);

		if (status_value(MobStatParser::PIMMUNE) && !magic_damage::pierces(randomizer.next_int(100), 0))
			return std::pair<int32_t, bool>(1, false);

		// Snipe's shot neither misses nor meets defense or guards.
		if (attack.skill == SkillId::SNIPE && attack.type == Attack::Type::RANGED)
			return std::pair<int32_t, bool>(shot_path::snipe_damage(randomizer.next_real(10000.0)), true);

		if (!blow_hits(attack))
			return std::pair<int32_t, bool>(0, false);

		bool swing = blow.motion == SWING || forced_swing(attack.skill, blow.job, index, randomizer.next_real(1.0));
		Formula formula = physical_damage::formula(blow.weapon, blow.job, attack.skill, blow.motion, swing, blow.stats,
												   least(blow.mastery));
		double damage = raw(formula, physical_damage::attack(blow.attack + blow.ammo), randomizer.next_real(1.0));
		damage *= capsule(attack.skill, attack.bullet);

		// Each level the monster is above the player takes a percent off.
		if (level > attack.playerlevel)
			damage = (100 - (level - attack.playerlevel)) * damage * 0.01;

		if (attack.skill && attack.element)
			damage = physical_damage::attribute(damage, elements[attack.element % magic_damage::ELEMENTS],
												blow.amplify * 0.01);

		damage = charged(damage, blow.charge, elements);

		if (defended(attack.skill))
			damage = defend(damage, defense(), randomizer.next_real(1.0));

		Critical critical = blow.critical;

		if (blow.stunned.source && status_value(MobStatParser::STUN))
			critical = stun(critical, blow.stunned.rate, blow.stunned.damage);

		bool second_strike = attack.display & assassinate::SECOND_STRIKE;
		bool iscritical = physical_damage::critical(critical, attack.skill, randomizer.next_real(1.0), second_strike);
		damage = scaled(damage, static_cast<int32_t>(damage), blow.scale, blow.combo, iscritical, critical.damage);

		// Sacrifice goes through a Power Guard Up.
		if (attack.skill != static_cast<int32_t>(SkillId::SACRIFICE))
			damage = magic_damage::guarded(damage, status_value(MobStatParser::PGUARD_UP));

		// Shadow Partner's set does a share of the first's, and crits
		// with it (0x7902d1).
		size_t half = hits.size() / 2;

		if (blow.partner && index >= half && half > 0)
		{
			const std::pair<int32_t, bool> &first = hits[index - half];
			damage = first.first * blow.partner / 100;
			iscritical = first.second;
		}

		// Assassinate builds up in Dark Sight (0x79028f).
		damage *= blow.stealth;

		// A held skill hits harder the longer it was charged (0x79048c).
		damage = prepare::charge_percent(attack.skill, attack.charge) * damage * 0.01;

		// Only a critical hit gets through Hard Skin (0x7904ba).
		if (status_value(MobStatParser::HARD_SKIN) && !iscritical)
			return std::pair<int32_t, bool>(0, false);

		if (blow.berserk)
			damage = (blow.berserk + 100) * damage * 0.01;

		damage = barrage(damage, attack.skill, index);

		return std::pair<int32_t, bool>(finish(damage), iscritical);
	}

	int32_t Mob::next_magic_damage(const Attack &attack) const
	{
		using namespace magic_damage;

		// Disable turns every hit away, and Magic Immunity all but 1 in
		// 100 to 1 damage (FUN_00791617).
		if (status_value(MobStatParser::DISABLE))
			return 0;

		if (status_value(MobStatParser::MIMMUNE) && !pierces(randomizer.next_int(100), 0))
			return 1;

		double hitroll = randomizer.next_real(1.0);
		double accuracy = magic_damage::accuracy(attack.intelligence, attack.luck, attack.playerlevel, level);

		if (!hits(accuracy, avoid + status_value(MobStatParser::EVA), hitroll))
			return 0;

		double roll = randomizer.next_real(1.0);
		double damage = attack.heal
				? heal::base(attack.heal, attack.magic, attack.intelligence, attack.luck, attack.healed, roll)
				: base(attack.magic, attack.intelligence, attack.mastery, attack.matk, roll);
		damage = damage * attack.amplify * 0.01;
		damage = elemental(damage, attack.skill, attack.element, elements);
		damage = defend(damage, mdef + status_value(MobStatParser::MDR), randomizer.next_real(1.0));
		damage = guarded(damage, status_value(MobStatParser::MGUARD_UP));
		damage = prepare::charge_percent(attack.skill, attack.charge) * damage * 0.01;

		// Hard Skin stops all but critical spells, and only an Evan's are.
		if (status_value(MobStatParser::HARD_SKIN))
			return 0;

		damage = percent(damage, attack.elemboost);
		damage = weakened(damage, status_value(MobStatParser::WEAKNESS));

		return finish(damage);
	}

	void Mob::apply_damage(int32_t damage, bool toleft)
	{
		hitsound.play();

		if (pending_hits > 0)
			pending_hits--;

		if (dying && !leaving && stance != Stance::DIE)
		{
			if (pending_hits == 0)
				apply_death();
		} else if (control && is_alive() && damage >= knockback && !acting)
		{
			// It shows the hit at once; the AI flinches next update.
			show_stance(Stance::HIT, toleft);
			attacking.reset();
			casting.reset();
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
		attacking.reset();
		casting.reset();
		acting = true;
		flinch = Flinch{ false, user };
	}

	MobAttack Mob::create_touch_attack() const
	{
		if (!deals_touch_damage())
			return MobAttack();

		return create_touch_attack_base();
	}

	MobAttack Mob::create_touch_attack_base() const
	{
		// A touch hits with the mob's own attack (FUN_009581a9).
		MobAttack attack(watk + status_value(MobStatParser::PAD), get_position(), id, oid);
		attack.matk = matk + status_value(MobStatParser::MAD);
		attack.level = level;
		attack.accuracy = accuracy + status_value(MobStatParser::ACC);
		attack.avoid = avoid;
		attack.maxhp = maxhp;
		attack.fixeddamage = fixeddamage;
		attack.boss = boss;
		attack.invincible = invincible;
		attack.powerup = status_value(MobStatParser::POWER_UP);
		attack.magicup = status_value(MobStatParser::MAGIC_UP);
		attack.blind = status_value(MobStatParser::BLIND);

		return attack;
	}

	int32_t Mob::status_value(uint8_t stat) const
	{
		auto found = statuses.find(stat);

		return found != statuses.end() ? static_cast<int32_t>(found->second.value) : 0;
	}

	int32_t Mob::counter(bool magic) const
	{
		auto found = statuses.find(magic ? MobStatParser::MCOUNTER : MobStatParser::PCOUNTER);

		if (found == statuses.end())
			return 0;

		return MobStatParser::counter(found->second, randomizer.next_int<uint32_t>(UINT32_MAX),
									  randomizer.next_int<uint32_t>(UINT32_MAX));
	}

	void Mob::apply_death()
	{
		show_stance(Stance::DIE, flip);
		diesound.play();
		tremble_on_death();
		dying = true;
	}

	void Mob::tremble_on_death() const
	{
		switch (id)
		{
			case 8800002:
				Stage::get().tremble(30, false, 0, 0, false);
				break;
			case 8810026:
				Stage::get().tremble(10, false, 0, 4000, true);
				break;
			case 8810018:
				Stage::get().tremble(5, false, 0, 3000, true);
				break;
			case 8830003:
				Stage::get().tremble(10, false, 0, 3000, false);
				break;
			default:
				break;
		}
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