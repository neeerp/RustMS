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
#include "MapSummons.h"

#include "../../Constants.h"

#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	using namespace summon_rules;

	namespace
	{
		// How long a hit makes it blink, and a miss keeps it from another
		// (FUN_007a6ebe).
		constexpr int16_t HIT_BLINK = 1500;

		Movement at(double x, double y, double vx, double vy, uint8_t action, uint8_t command = 0)
		{
			return Movement(Movement::ABSOLUTE, command, static_cast<int16_t>(std::lround(x)),
							static_cast<int16_t>(std::lround(y)), static_cast<int16_t>(std::lround(vx)),
							static_cast<int16_t>(std::lround(vy)), 0, action, 0);
		}
	}

	Summon::Summon(int32_t owner, int32_t skill, uint8_t level, Point<int16_t> position, uint8_t action,
				   uint8_t moves, uint8_t assist, bool appear, bool local)
			: owner(owner), skill(skill), level(level), moves(moves), assist(assist), local(local), x(position.x()),
			  y(position.y()), left(action & 1), action(action),
			  follower{ x, y, left }, circler{ x, y, left }, anchor(position),
			  shown({ x, y, 0.0, 0.0, 0, action })
	{
		if (appear)
			oneshot = SUMMONED;

		phobj.set_x(x);
		phobj.set_y(y);
		recorder.reset(at(x, y, 0, 0, action));
	}

	const Animation &Summon::animation_for(Action wanted)
	{
		auto iter = animations.find(wanted);

		if (iter != animations.end())
			return iter->second;

		// A missing animation falls back to fly for a flier, else to stand
		// (FUN_007a3acb).
		nl::node src = nl::nx::skill[string_format::extend_id(skill / 10000, 3) + ".img"]["skill"]
		[string_format::extend_id(skill, 7)]["summon"];
		nl::node node = src[action_name(wanted)];

		if (!node)
			node = src[flies(moves) ? "fly" : "stand"];

		if (!node)
			node = src["stand"];

		return animations.emplace(wanted, Animation(node)).first->second;
	}

	void Summon::show(Action wanted)
	{
		if (wanted == showing)
			return;

		animation_for(wanted);
		animations[wanted].reset();
		showing = wanted;
	}

	void Summon::play(Action played)
	{
		if (dying)
			return;

		oneshot = played;
		showing = NONE;
	}

	void Summon::receive(const std::vector<Movement> &elements)
	{
		if (!local)
			shown.receive(elements);
	}

	void Summon::hurt(int32_t damage)
	{
		hitcountdown = damage > 0 ? HIT_BLINK : -HIT_BLINK;

		if (damage > 0)
			play(HIT);
	}

	bool Summon::remove(bool animated)
	{
		if (!animated)
		{
			removed = true;

			return false;
		}

		oneshot = DIE;
		showing = NONE;
		dying = true;

		return true;
	}

	uint8_t Summon::move_action() const
	{
		return action;
	}

	std::optional<move_path::Path> Summon::update(const Physics &physics, Point<int16_t> owner, bool held,
												  Randomizer &random)
	{
		if (hitcountdown > 0)
			hitcountdown = static_cast<int16_t>(std::max(0, hitcountdown - Constants::TIMESTEP));
		else if (hitcountdown < 0)
			hitcountdown = static_cast<int16_t>(std::min(0, hitcountdown + Constants::TIMESTEP));

		std::optional<move_path::Path> sent;

		if (!dying && local && moves != STATIONARY)
		{
			double ms = Constants::TIMESTEP;
			double lastx = x;
			double lasty = y;
			bool teleported = false;

			if (moves != CIRCLE_STATIONARY && lost(owner.x() - x, owner.y() - y))
			{
				x = owner.x();
				y = owner.y();
				follower = Follower{ x, y, left };
				circler = Circler{ x, y, left };
				teleported = true;
			}

			uint8_t stance = 0;

			if (moves == CIRCLE_FOLLOW || moves == CIRCLE_STATIONARY)
			{
				Point<int16_t> around = moves == CIRCLE_FOLLOW ? owner : anchor;
				circle(circler, around.x(), around.y(), moves == CIRCLE_STATIONARY,
					   [&](int32_t n) { return random.next_int(n); }, ms);
				x = circler.x;
				y = circler.y;
				left = circler.left;
				stance = 6;
			} else
			{
				follow(follower, owner.x(), owner.y(), held, is_soul(skill) ? 42 : 70, ms);
				x = follower.x;
				y = follower.y;
				left = follower.left;
				stance = follower.moving ? 1 : 0;
			}

			action = static_cast<uint8_t>(stance << 1 | (left ? 1 : 0));

			if (teleported)
			{
				recorder.add(at(x, y, 0, 0, action, 4));
			} else
			{
				Movement now = at(x, y, (x - lastx) * 1000 / ms, (y - lasty) * 1000 / ms, action);
				recorder.step(now, Constants::TIMESTEP);
			}

			if (recorder.ready(true))
			{
				move_path::Path path = recorder.flush(true);

				if (!path.idle)
					sent = std::move(path);
			}
		} else if (!dying && !local)
		{
			shown.step(Constants::TIMESTEP);

			const move_path::Player::State &now = shown.get_state();
			x = now.x;
			y = now.y;
			action = now.stance;
			left = action & 1;
		}

		Action wanted = pick(oneshot, action);
		show(wanted);

		if (animations[wanted].update() && oneshot != NONE)
		{
			if (dying)
				removed = true;

			oneshot = NONE;
		}

		if (!flies(moves))
		{
			phobj.set_x(x);
			phobj.set_y(y);
			physics.get_fht().update_fh(phobj);
		}

		return sent;
	}

	void Summon::draw(double viewx, double viewy, float alpha) const
	{
		auto iter = animations.find(showing);

		if (iter == animations.end())
			return;

		// It blinks while a hit holds it (FUN_007a41d4).
		float opacity = hitcountdown > 0 && hitcountdown / 120 % 2 ? 0.5f : 1.0f;
		Point<int16_t> position(static_cast<int16_t>(std::lround(x + viewx)),
								static_cast<int16_t>(std::lround(y + viewy)));
		iter->second.draw(DrawArgument(position, !left, opacity), alpha);
	}

	bool Summon::gone() const
	{
		return removed;
	}

	int32_t Summon::get_owner() const
	{
		return owner;
	}

	int32_t Summon::get_skill() const
	{
		return skill;
	}

	uint8_t Summon::get_level() const
	{
		return level;
	}

	uint8_t Summon::get_moves() const
	{
		return moves;
	}

	uint8_t Summon::get_assist() const
	{
		return assist;
	}

	Point<int16_t> Summon::get_position() const
	{
		return Point<int16_t>(static_cast<int16_t>(std::lround(x)), static_cast<int16_t>(std::lround(y)));
	}

	bool Summon::is_left() const
	{
		return left;
	}

	void Summon::set_left(bool l)
	{
		left = l;
		action = static_cast<uint8_t>((action & ~1) | (l ? 1 : 0));
		follower.left = l;
		circler.left = l;
	}

	bool Summon::is_local() const
	{
		return local;
	}

	bool Summon::is_flying() const
	{
		return flies(moves);
	}

	int8_t Summon::get_layer() const
	{
		return phobj.fhlayer;
	}

	bool Summon::is_idle() const
	{
		return !dying && oneshot == NONE && can_act(moves, skill);
	}

	bool Summon::can_be_hit() const
	{
		return !dying && !removed && !can_act(moves, skill) && hitcountdown == 0;
	}

	Rectangle<int16_t> Summon::get_body() const
	{
		auto iter = animations.find(showing);
		Point<int16_t> position = get_position();

		if (iter == animations.end())
			return Rectangle<int16_t>(position, position);

		Rectangle<int16_t> bounds = iter->second.get_bounds();

		// The art faces left; facing right mirrors it.
		if (left)
			return Rectangle<int16_t>(position.x() + bounds.left(), position.x() + bounds.right(),
									  position.y() + bounds.top(), position.y() + bounds.bottom());

		return Rectangle<int16_t>(position.x() - bounds.right(), position.x() - bounds.left(),
								  position.y() + bounds.top(), position.y() + bounds.bottom());
	}

	void MapSummons::spawn(int32_t owner, int32_t oid, int32_t skill, uint8_t level, Point<int16_t> position,
						   uint8_t action, uint8_t moves, uint8_t assist, bool appear, bool local)
	{
		auto iter = summons.find(oid);

		if (iter != summons.end())
		{
			if (local && iter->second.is_local())
				return;

			summons.erase(iter);
		}

		summons.emplace(std::piecewise_construct, std::forward_as_tuple(oid),
						std::forward_as_tuple(owner, skill, level, position, action, moves, assist, appear, local));
	}

	int32_t MapSummons::remove(int32_t oid, uint8_t reason)
	{
		auto iter = summons.find(oid);

		if (iter == summons.end())
			return 0;

		Summon &summon = iter->second;
		int32_t skill = summon.get_skill();
		bool animated = reason == 0 || reason == 3 || reason == 4 || (reason == 2 && is_puppet(skill));

		if (!summon.remove(animated))
			summons.erase(iter);

		return skill;
	}

	void MapSummons::remove_owner(int32_t owner)
	{
		for (auto iter = summons.begin(); iter != summons.end();)
		{
			if (iter->second.get_owner() == owner)
				iter = summons.erase(iter);
			else
				++iter;
		}
	}

	void MapSummons::move(int32_t oid, const std::vector<Movement> &elements)
	{
		if (Summon *summon = get(oid))
			summon->receive(elements);
	}

	void MapSummons::act(int32_t oid, uint8_t action)
	{
		if (Summon *summon = get(oid))
			summon->play(static_cast<Action>(action & 0x7F));
	}

	void MapSummons::hurt(int32_t oid, int32_t damage)
	{
		Summon *summon = get(oid);

		if (!summon)
			return;

		summon->hurt(damage);

		// Its number over it, as a mob's on the player (FUN_007a70e6).
		numbers.emplace_back(DamageNumber::Type::TOPLAYER, std::max(damage, 0));
		numbers.back().set_anchor(summon->get_position() - Point<int16_t>(0, 50));
	}

	void MapSummons::update(const Physics &physics, Point<int16_t> owner, bool held)
	{
		for (auto iter = summons.begin(); iter != summons.end();)
		{
			Summon &summon = iter->second;

			if (std::optional<move_path::Path> path = summon.update(physics, owner, held, random))
				MoveSummonPacket(iter->first, *path).dispatch();

			if (summon.gone())
				iter = summons.erase(iter);
			else
				++iter;
		}

		numbers.remove_if([](DamageNumber &number) { return number.update(); });
	}

	void MapSummons::draw(int8_t layer, double viewx, double viewy, float alpha) const
	{
		for (const auto &iter : summons)
		{
			const Summon &summon = iter.second;

			if (summon.is_flying() ? layer == -1 : summon.get_layer() == layer)
				summon.draw(viewx, viewy, alpha);
		}

		if (layer == -1)
			for (const DamageNumber &number : numbers)
				number.draw(viewx, viewy, alpha);
	}

	void MapSummons::clear()
	{
		summons.clear();
		numbers.clear();
	}

	std::vector<std::pair<int32_t, Summon *>> MapSummons::get_local()
	{
		std::vector<std::pair<int32_t, Summon *>> local;

		for (auto &iter : summons)
			if (iter.second.is_local())
				local.emplace_back(iter.first, &iter.second);

		return local;
	}

	Summon *MapSummons::get(int32_t oid)
	{
		auto iter = summons.find(oid);

		return iter == summons.end() ? nullptr : &iter->second;
	}
}
