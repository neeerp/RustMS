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
#include "MapMobs.h"
#include "FieldLimit.h"
#include "Mob.h"

#include <algorithm>
#include <iostream>
#include <map>

namespace ms
{
	void MapMobs::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		mobs.draw(layer, viewx, viewy, alpha);
	}

	void MapMobs::update(const Physics &physics, const Mob::Target &target)
	{
		for (; !spawns.empty(); spawns.pop())
		{
			const MobSpawn &spawn = spawns.front();

			if (Optional<Mob> mob = mobs.get(spawn.get_oid()))
			{
				int8_t mode = spawn.get_mode();

				if (mode > 0)
					mob->set_control(mode);

				mob->makeactive();
			} else
			{
				mobs.add(spawn.instantiate(physics));
			}
		}

		for (auto &mmo : mobs)
		{
			Optional<Mob> mob = mmo.second.get();

			if (mob)
				mob->set_target(target);
		}

		mobs.update(physics);
	}

	std::vector<Mob::Landed> MapMobs::take_attacks()
	{
		std::vector<Mob::Landed> landed;

		for (auto &mmo : mobs)
		{
			if (Optional<Mob> mob = mmo.second.get())
			{
				for (Mob::Landed &hit : mob->take_attacks())
					landed.push_back(std::move(hit));
			}
		}

		return landed;
	}

	void MapMobs::spawn(MobSpawn &&spawn)
	{
		spawns.emplace(std::move(spawn));
	}

	void MapMobs::remove(int32_t oid, int8_t animation)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->kill(animation);
	}

	void MapMobs::clear()
	{
		mobs.clear();
	}

	void MapMobs::set_control(int32_t oid, bool control)
	{
		int8_t mode = control ? 1 : 0;

		if (Optional<Mob> mob = mobs.get(oid))
			mob->set_control(mode);
	}

	void MapMobs::acknowledge(int32_t oid, int16_t moveid, bool nextattack, int16_t mp, uint8_t skill,
							  uint8_t skilllevel)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->acknowledge(moveid, nextattack, mp, skill, skilllevel);
	}

	void MapMobs::show_damage(int32_t oid, int32_t damage)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->show_damage(damage);
	}

	void MapMobs::show_reflect(int32_t oid, uint8_t percent, int32_t damage, bool physical)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->show_reflect(percent, damage, physical);
	}

	int32_t MapMobs::counter(int32_t oid, bool magic) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->counter(magic);

		return 0;
	}

	void MapMobs::send_mobhp(int32_t oid, int8_t percent, uint16_t playerlevel)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->show_hp(percent, playerlevel);
	}

	void MapMobs::set_statuses(int32_t oid, const std::vector<MobStatParser::Status> &statuses)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->set_statuses(statuses, true);
	}

	void MapMobs::reset_statuses(int32_t oid, const BuffParser::Flags &flags)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->reset_statuses(flags);
	}

	void MapMobs::catch_mob(int32_t oid, Point<int16_t> user)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->catch_by(user);
	}

	void MapMobs::send_movement(int32_t oid, Point<int16_t> start, std::vector<Movement> &&movements,
								int8_t action, bool left, int32_t skilldata)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->send_movement(start, std::move(movements), action, left, skilldata);
	}

	void MapMobs::send_attack(AttackResult &result, const Attack &attack, const std::vector<int32_t> &targets,
							  uint8_t mobcount)
	{
		for (auto &target : targets)
		{
			if (Optional<Mob> mob = mobs.get(target))
			{
				result.damagelines.emplace_back(target, mob->calculate_damage(attack));
				result.mobcount++;

				if (result.mobcount == 1)
					result.first_oid = target;

				if (result.mobcount == mobcount)
					result.last_oid = target;
			}
		}
	}

	void
	MapMobs::apply_damage(int32_t oid, int32_t damage, bool toleft, const AttackUser &user, const SpecialMove &move)
	{
		if (Optional<Mob> mob = mobs.get(oid))
		{
			mob->apply_damage(damage, toleft);

			if (user.calms)
				mob->calm();

			// TODO: Maybe move this into the method above too?
			move.apply_hiteffects(user, *mob);
		}
	}

	void MapMobs::apply_summon_damage(int32_t oid, int32_t damage, bool toleft, const Animation &hit)
	{
		if (Optional<Mob> mob = mobs.get(oid))
		{
			mob->apply_damage(damage, toleft);
			mob->show_hit(hit, mob->get_body(), toleft);
		}
	}

	void MapMobs::expect_hits(int32_t oid, int32_t count)
	{
		if (Optional<Mob> mob = mobs.get(oid))
			mob->expect_hits(count);
	}

	bool MapMobs::contains(int32_t oid) const
	{
		return mobs.contains(oid);
	}

	int32_t MapMobs::find_colliding(const MovingObject &moveobj, int32_t fieldlimit, bool hidden) const
	{
		Range<int16_t> horizontal = Range<int16_t>(moveobj.get_last_x(), moveobj.get_x());
		Range<int16_t> vertical = Range<int16_t>(moveobj.get_last_y(), moveobj.get_y());

			Rectangle<int16_t> player_rect = {
					horizontal.smaller(),
					horizontal.greater(),
					static_cast<int16_t>(vertical.smaller() - 50),
					vertical.greater()
			};

		return find_touching(player_rect, fieldlimit, hidden);
	}

	int32_t MapMobs::find_touching(const Rectangle<int16_t> &area, int32_t fieldlimit, bool hidden) const
	{
		auto iter = std::find_if(
				mobs.begin(),
				mobs.end(),
				[&](auto &mmo)
				{
					Optional<Mob> mob = mmo.second.get();
					return mob && mob->is_alive() && mob->deals_touch_damage() && mob->is_in_range(area)
							&& field_limit::may_touch(fieldlimit, hidden, mob->get_id());
				}
		);

		if (iter == mobs.end())
			return 0;

		return iter->second->get_oid();
	}

	int32_t MapMobs::find_catchable(const Rectangle<int16_t> &area, int32_t mobid) const
	{
		int32_t found = 0;
		uint32_t first = UINT32_MAX;

		for (auto &mmo : mobs)
		{
			Optional<Mob> mob = mmo.second.get();

			if (!mob || !mob->is_alive() || mob->get_id() != mobid || !mob->is_in_range(area))
				continue;

			uint32_t order = mobs.entry_order(mmo.first);

			if (order < first || !found)
			{
				found = mmo.first;
				first = order;
			}
		}

		return found;
	}

	void MapMobs::aggro_puppet(int32_t oid, Point<int16_t> position)
	{
		for (auto &mmo : mobs)
			if (Optional<Mob> mob = mmo.second.get())
				mob->aggro_puppet(oid, position);
	}

	void MapMobs::release_puppet(int32_t oid)
	{
		for (auto &mmo : mobs)
			if (Optional<Mob> mob = mmo.second.get())
				mob->release_puppet(oid);
	}

	int32_t MapMobs::roll_puppet_damage(int32_t oid, int32_t level) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->roll_puppet_damage(level);

		return 0;
	}

	MobAttack MapMobs::create_attack(int32_t oid) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->create_touch_attack();
		else
			return {};
	}

	Point<int16_t> MapMobs::get_mob_position(int32_t oid) const
	{
		if (auto mob = mobs.get(oid))
			return mob->get_position();
		else
			return Point<int16_t>(0, 0);
	}

	Point<int16_t> MapMobs::get_mob_head_position(int32_t oid) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->get_head_position();
		else
			return Point<int16_t>(0, 0);
	}

	int32_t MapMobs::get_mob_id(int32_t oid) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->get_id();

		return 0;
	}

	std::vector<std::pair<Point<int16_t>, int32_t>> MapMobs::take_hurts()
	{
		std::vector<std::pair<Point<int16_t>, int32_t>> hurts;

		for (auto &mmo : mobs)
		{
			if (Optional<Mob> mob = mmo.second.get())
			{
				for (int32_t number : mob->take_hurts())
					hurts.emplace_back(mob->get_head_position(), number);
			}
		}

		return hurts;
	}

	Rectangle<int16_t> MapMobs::get_mob_body(int32_t oid) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->get_body();
		else
			return Rectangle<int16_t>();
	}

	std::vector<meso_explosion::Mob> MapMobs::explosion_targets() const
	{
		std::vector<std::pair<uint32_t, meso_explosion::Mob>> found;

		for (auto &mmo : mobs)
		{
			auto mob = static_cast<const Mob *>(mmo.second.get());

			if (mob && mob->is_alive())
				found.push_back({ mobs.entry_order(mmo.first), { mmo.first, mob->get_body() } });
		}

		std::sort(found.begin(), found.end(), [](auto &a, auto &b) { return a.first < b.first; });

		std::vector<meso_explosion::Mob> targets;

		for (auto &[entry, mob] : found)
			targets.push_back(mob);

		return targets;
	}

	std::vector<int32_t> MapMobs::explode(int32_t oid, int32_t x, const std::vector<int32_t> &mesos) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->explode(x, mesos);

		return std::vector<int32_t>(mesos.size(), 0);
	}

	int32_t MapMobs::shown_explosion(int32_t oid, int32_t number) const
	{
		if (Optional<const Mob> mob = mobs.get(oid))
			return mob->shown_explosion(number);

		return number;
	}

	MapObjects *MapMobs::get_mobs()
	{
		return &mobs;
	}
}
