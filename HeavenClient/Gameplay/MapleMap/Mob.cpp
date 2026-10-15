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

#include "../../Character/TouchDamage.h"
#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdlib>

namespace ms
{
	Mob::Mob(int32_t oi, int32_t mid, int8_t mode, int8_t st, uint16_t fh, bool newspawn, int8_t tm,
			 Point<int16_t> position) : MapObject(oi)
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
		touchdamage = info["bodyAttack"].get_bool();
		undead = info["undead"].get_bool();
		noflip = info["noFlip"].get_bool();
		notattack = info["notAttack"].get_bool();
		canjump = src["jump"].size() > 0;
		canfly = src["fly"].size() > 0;
		canmove = src["move"].size() > 0 || canfly;

		if (canfly)
		{
			animations[Stance::STAND] = src["fly"];
			animations[Stance::MOVE] = src["fly"];
		} else
		{
			animations[Stance::STAND] = src["stand"];
			animations[Stance::MOVE] = src["move"];
		}

		animations[Stance::JUMP] = src["jump"];
		animations[Stance::HIT] = src["hit1"];
		animations[Stance::DIE] = src["die1"];

		name = std::string(nl::nx::string["Mob.img"][std::to_string(mid)]["name"]);

		nl::node sndsrc = nl::nx::sound["Mob.img"][strid];

		hitsound = sndsrc["Damage"];
		diesound = sndsrc["Die"];

		speed += 100;
		speed *= 0.001f;

		flyspeed += 100;
		flyspeed *= 0.0005f;

		if (canfly)
			phobj.type = PhysicsObject::Type::FLYING;

		id = mid;
		team = tm;
		set_position(position);
		hastarget = false;
		targettime = 0;
		aitime = 0;
		thinktimer = 0;
		movedir = 0;
		set_control(mode);
		phobj.fhid = fh;
		phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);

		hppercent = 0;
		dying = false;
		dead = false;
		fading = false;
		set_stance(st);
		flydirection = STRAIGHT;
		counter = 0;
		knockback_ticks = 0;
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

		if (control && stance == Stance::STAND && canfly)
			next_move();
	}

	void Mob::set_stance(uint8_t stancebyte)
	{
		flip = (stancebyte % 2) == 0;

		if (!flip)
			stancebyte -= 1;

		if (stancebyte < Stance::MOVE)
			stancebyte = Stance::MOVE;

		set_stance(static_cast<Stance>(stancebyte));
	}

	void Mob::set_stance(Stance newstance)
	{
		// A hit nudges the mob back a few pixels; it doesn't slide away.
		if (newstance == Stance::HIT)
			knockback_ticks = 3;

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

		if (!dying)
		{
			if (!canfly)
			{
				if (phobj.is_flag_not_set(PhysicsObject::Flag::TURNATEDGES))
				{
					flip = !flip;
					phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);

					if (stance == Stance::HIT)
						set_stance(Stance::STAND);
				}
			}

			switch (stance)
			{
				case Stance::MOVE:
					if (canfly)
					{
						phobj.hforce = flip ? flyspeed : -flyspeed;

						switch (flydirection)
						{
							case FlyDirection::UPWARDS:
								phobj.vforce = -flyspeed;
								break;
							case FlyDirection::DOWNWARDS:
								phobj.vforce = flyspeed;
								break;
						}
					} else
					{
						phobj.hforce = flip ? speed : -speed;
					}

					break;
				case Stance::HIT:
					if (canmove && knockback_ticks > 0)
					{
						knockback_ticks--;

						double KBFORCE = phobj.onground ? 0.2 : 0.1;
						phobj.hforce = flip ? -KBFORCE : KBFORCE;
					}

					break;
				case Stance::JUMP:
					phobj.vforce = -5.0;
					break;
			}

			physics.move_object(phobj);

			if (control && canfly)
			{
				counter++;

				bool next;

				switch (stance)
				{
					case Stance::HIT:
						next = counter > 200;
						break;
					default:
						next = aniend && counter > 200;
						break;
				}

				if (next)
				{
					next_move();
					update_movement();
					counter = 0;
				}
			} else if (control)
			{
				// v83 gives up a chase 3 to 4 minutes after aggro (FUN_007f7f02).
				if (hastarget)
				{
					targettime -= Constants::TIMESTEP;

					if (targettime < 0)
						hastarget = false;
				}

				switch (stance)
				{
					case Stance::HIT:
						// v83 holds a hit mob still, then leaves it standing
						// until its AI next decides.
						if (++counter > 200)
						{
							set_stance(Stance::STAND);
							update_movement();
							counter = 0;
						}

						break;
					case Stance::JUMP:
						if (phobj.onground)
							walk(movedir);

						break;
					default:
						// The mob vecctrl steps its AI in 90 ms steps (FUN_007f72d9).
						for (aitime += Constants::TIMESTEP; aitime >= 90; aitime -= 90)
							think();

						break;
				}
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

	void Mob::next_move()
	{
		if (canmove)
		{
			switch (stance)
			{
				case Stance::HIT:
				case Stance::STAND:
					set_stance(Stance::MOVE);
					flip = randomizer.next_bool();
					break;
				case Stance::MOVE:
				case Stance::JUMP:
					if (canjump && phobj.onground && randomizer.below(0.25f))
					{
						set_stance(Stance::JUMP);
					} else
					{
						switch (randomizer.next_int(3))
						{
							case 0:
								set_stance(Stance::STAND);
								break;
							case 1:
								set_stance(Stance::MOVE);
								flip = false;
								break;
							case 2:
								set_stance(Stance::MOVE);
								flip = true;
								break;
						}
					}

					break;
			}

			if (stance == Stance::MOVE && canfly)
				flydirection = randomizer.next_enum(FlyDirection::NUM_DIRECTIONS);
		} else
		{
			set_stance(Stance::STAND);
		}
	}

	void Mob::think()
	{
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
			set_stance(Stance::JUMP);
			update_movement();
			return;
		} else
		{
			thinktimer = (randomizer.next_int(2000) + 1000) / 90;
			movedir = static_cast<int8_t>(randomizer.next_int(3) - 1);
			walk(movedir);
		}

		thinktimer--;
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
			set_stance(Stance::MOVE);
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
		Point<int16_t> headpos = get_head_position(absp);

		effects.drawbelow(absp, alpha);

		if (!dead)
		{
			float interopc = opacity.get(alpha);

			animations.at(stance).draw(DrawArgument(absp, flip && !noflip, interopc), alpha);

			if (showhp)
			{
				namelabel.draw(absp);

				int16_t width = 0;

				for (const Texture& part : leveltag)
					width += part.width();

				Point<int16_t> tagpos = absp + Point<int16_t>(-namelabel.width() / 2 - width - 3, 5);

				for (const Texture& part : leveltag)
				{
					part.draw(tagpos);
					tagpos.shift_x(part.width());
				}

				if (!dying && hppercent > 0)
					hpbar.draw(headpos, hppercent);
			}
		}

		effects.drawabove(absp, alpha);
	}

	void Mob::set_control(int8_t mode)
	{
		control = mode > 0;

		// v83 (FUN_005d511d -> FUN_007f73d8) has a mob given with aggro
		// chase the local player for 180 to 240 s and decide at once.
		if (mode == 2)
		{
			hastarget = true;
			targettime = 180000 + randomizer.next_int(60000);
			thinktimer = 0;
			movedir = 0;
		} else if (mode == 0)
		{
			hastarget = false;
		}
	}

	void Mob::set_target(Point<int16_t> position)
	{
		targetpos = position;
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
		if (hppercent == 0)
		{
			int16_t delta = playerlevel - level;

			if (delta > 9)
			{
				namelabel.change_color(Color::Name::YELLOW);
				set_level_tag(1);
			}
			else if (delta < -9)
			{
				namelabel.change_color(Color::Name::RED);
				set_level_tag(2);
			}
		}

		if (percent > 100)
			percent = 100;
		else if (percent < 0)
			percent = 0;

		hppercent = percent;
		showhp.set_for(2000);
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
		} else if (control && is_alive() && damage >= knockback)
		{
			flip = toleft;
			counter = 170;
			set_stance(Stance::HIT);

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