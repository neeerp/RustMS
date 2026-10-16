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
#include "Combat.h"

#include "ChainLightning.h"
#include "DamageLayout.h"
#include "HitTiming.h"
#include "PointBlank.h"
#include "Prepare.h"
#include "ShotPath.h"

#include "../../Constants.h"
#include "../../Character/SkillId.h"
#include "../../Data/SkillData.h"
#include "../../IO/Messages.h"
#include "../../Util/Randomizer.h"

#include "../../Net/Packets/AttackAndSkillPackets.h"
#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Combat::Combat(Player &in_player, MapChars &in_chars, MapMobs &in_mobs, MapReactors &in_reactors,
				   const Physics &in_physics) : player(
			in_player), chars(in_chars), mobs(in_mobs), reactors(in_reactors), physics(in_physics),
																										attackresults(
																												[&](const AttackResult &attack)
																												{
																													apply_attack(
																															attack);
																												}),
																										bulleteffects(
																												[&](const Bullet &bullet)
																												{
																													apply_bullet_effect(
																															bullet);
																												}),
																										damageeffects(
																												[&](const DamageEffect &effect)
																												{
																													apply_damage_effect(
																															effect);
																												})
	{
	}

	void Combat::init()
	{
		nl::node basic = nl::nx::ui["Basic.img"];
		nl::node bars[2] = { basic["KeyDownBar"], basic["KeyDownBar1"] };

		for (size_t i = 0; i < 2; i++)
		{
			gauge_bar[i] = bars[i]["bar"];
			gauge_fill[i] = bars[i]["gauge"];
			// KeyDownBar1's graduation only links to KeyDownBar's.
			gauge_graduation[i] = bars[0]["graduation"];
		}
	}

	Texture Combat::gauge_bar[2];
	Texture Combat::gauge_fill[2];
	Texture Combat::gauge_graduation[2];

	void Combat::draw(double viewx, double viewy, float alpha) const
	{
		for (auto &bullet : bullets)
			bullet.draw(viewx, viewy, alpha);

		for (auto &dn : damagenumbers)
			dn.draw(viewx, viewy, alpha);

		draw_gauge(viewx, viewy, alpha);
	}

	void Combat::draw_gauge(double viewx, double viewy, float alpha) const
	{
		if (!preparing || !prepare::shows_gauge(preparing->skill))
			return;

		// The bar, filled a column at a time, under its graduation, with its
		// top left 36 px left of the player and 100 px up (0x95c375, 0x95c3e3).
		int32_t kind = prepare::gauge_kind(preparing->skill);
		Point<int16_t> topleft = player.get_phobj().get_absolute(viewx, viewy, alpha) + Point<int16_t>(-36, -100);

		gauge_bar[kind].draw(topleft + gauge_bar[kind].get_origin());

		int32_t end = prepare::gauge(preparing->skill, preparing->held);

		for (int32_t x = prepare::GAUGE_START; x <= end; x++)
			gauge_fill[kind].draw(topleft + Point<int16_t>(x, 2) + gauge_fill[kind].get_origin());

		gauge_graduation[kind].draw(topleft + gauge_graduation[kind].get_origin());
	}

	void Combat::update()
	{
		now += Constants::TIMESTEP;
		update_prepare();

		attackresults.update();
		bulleteffects.update();
		damageeffects.update();

		bullets.remove_if(
				[](Bullet &bullet)
				{
					return bullet.update();
				}
		);

		damagenumbers.remove_if(
				[](DamageNumber &dn)
				{
					return dn.update();
				}
		);
	}

	void Combat::use_move(int32_t move_id)
	{
		if (prepare::held(move_id) || prepare::timed(move_id))
		{
			// A skill that keeps firing whacks a mob right in front of the
			// player, or stabs while prone, instead (0x96ae26).
			if (prepare::repeats(move_id) && !preparing && player.can_attack())
			{
				int16_t reach = 0;

				if (player.is_prone() || find_whack(player.prepare_attack(0), reach) != Stance::Id::NONE)
				{
					use_move(0);
					return;
				}
			}

			const Skill &skill = get_skill(move_id);

			if (can_use_move(skill))
				start_prepare(skill);

			return;
		}

		const SpecialMove &move = get_move(move_id);

		if (!can_use_move(move))
			return;

		// v83's spell handler swings a regular attack instead when the player
		// is prone (0x969b10).
		if (move.is_skill() && player.is_prone() && SkillData::get(move_id).is_spell())
			apply_move(regularattack);
		else
			apply_move(move);
	}

	std::optional<teleport::Spot> Combat::use_teleport(int32_t move_id, bool allowed)
	{
		const SpecialMove &move = get_move(move_id);

		if (!can_use_move(move) || !allowed)
			return std::nullopt;

		// v83 teleports only from a platform, the way the arrow keys point,
		// reversed while confused (FUN_00957b74).
		const PhysicsObject &phobj = player.get_phobj();

		if (!phobj.onground || player.get_ladder())
			return std::nullopt;

		int32_t dx = player.is_key_down(KeyAction::Id::RIGHT) - player.is_key_down(KeyAction::Id::LEFT);
		int32_t dy = player.is_key_down(KeyAction::Id::DOWN) - player.is_key_down(KeyAction::Id::UP);

		if (player.has_buff(Buffstat::Id::CONFUSE))
		{
			dx = -dx;
			dy = -dy;
		}

		int32_t level = player.get_skills().get_level(move_id);
		int32_t range = SkillData::get(move_id).get_stats(level).shotrange;
		Point<int16_t> position = player.get_position();
		std::optional<Rectangle<int16_t>> area = teleport::mob_area(position, dx, dy, range);

		if (!area)
			return std::nullopt;

		MapObjects *objs = mobs.get_mobs();
		std::vector<teleport::Mob> nearby;

		for (auto &mmo : *objs)
		{
			const Mob *mob = static_cast<const Mob *>(mmo.second.get());

			if (mob && mob->is_alive())
				nearby.push_back({ mob->get_body(), objs->entry_order(mob->get_oid()), mob->blocks_teleport() });
		}

		if (teleport::mob_in_way(*area, nearby))
			return std::nullopt;

		uint16_t group = phobj.fhgroup;
		auto clear = [&](Point<int16_t> from, Point<int16_t> to)
		{
			return !shot_path::blocked(from, to, find_platforms(from, to), group);
		};

		std::optional<teleport::Spot> target = teleport::destination(position, phobj.fhid, dx, dy, range, find_floors(), clear);

		if (!target || !request_skill(move))
			return std::nullopt;

		return target;
	}

	bool Combat::can_use_move(const SpecialMove &move) const
	{
		// v83 takes no other move while a skill is held (0x96a86e).
		if (preparing || !player.can_attack())
			return false;

		SpecialMove::ForbidReason reason = player.can_use(move);
		Weapon::Type weapontype = player.get_stats().get_weapontype();

		if (reason != SpecialMove::ForbidReason::FBR_NONE)
		{
			ForbidSkillMessage(reason, weapontype).drop();
			return false;
		}

		// v83 says why it won't attack bare-handed (FUN_0095b85a), once
		// DoActiveSkill has let the skill through. Buffs need no weapon.
		if (move.is_attack() && !player.get_look().get_equips().has_weapon())
		{
			InChatMessage(Messages::ATTACK_NOWEAPON).drop();
			return false;
		}

		return true;
	}

	bool Combat::request_skill(const SpecialMove &move)
	{
		// v83 uses a skill that doesn't attack only when it may ask the
		// server, as for an item (FUN_0096b140).
		if (!player.can_request(ExclRequest::SKILL_WAIT))
			return false;

		move.apply_useeffects(player);
		move.apply_actions(player, Attack::Type::MAGIC);

		int32_t moveid = move.get_id();
		int32_t level = player.get_skills().get_level(moveid);
		player.request(UseSkillPacket(moveid, level), ExclRequest::SKILL_WAIT);

		return true;
	}

	void Combat::apply_move(const SpecialMove &move, int32_t charge, bool held)
	{
		if (move.is_attack())
		{
			Attack attack = player.prepare_attack(move.is_skill() ? move.get_id() : 0);
			attack.charge = charge;
			Stance::Id whack = Stance::Id::NONE;
			int16_t reach = 0;

			if (attack.type == Attack::Type::RANGED && !move.is_skill())
				whack = find_whack(attack, reach);

			if (whack != Stance::Id::NONE)
				attack = player.prepare_attack(0, true);

			// A held skill's shots keep the body as it is (0x954165).
			if (!held)
				move.apply_useeffects(player);

			if (whack != Stance::Id::NONE)
				player.attack(true, whack);
			else if (!held)
				move.apply_actions(player, attack.type);

			player.set_afterimage(move.get_id());

			move.apply_stats(player, attack);

			if (whack != Stance::Id::NONE)
				attack.range = point_blank::range(player.get_afterimage_range(whack));
			// Like v83, shoot from the first stack that covers the cost.
			const Inventory &inventory = player.get_inventory();

			if (attack.type == Attack::Type::RANGED && attack.bullet == inventory.get_bulletid())
			{
				attack.bulletslot = inventory.find_bulletslot(attack.bulletcost);
				attack.bullet = inventory.get_item_id(InventoryType::Id::USE, attack.bulletslot);
			}

			Point<int16_t> origin = attack.origin;
			Rectangle<int16_t> range = place_range(origin, attack.range, attack.hrange, attack.toleft);

			// This approach should also make it easier to implement PvP
			uint8_t mobcount = attack.mobcount;
			AttackResult result = attack;

			MapObjects *mob_objs = mobs.get_mobs();
			MapObjects *reactor_objs = reactors.get_reactors();

			// As in v83, ranged attacks hit the mobs closest to the player, while
			// melee and magic attacks hit them in the order they entered the map.
			bool closest_first = attack.type == Attack::Type::RANGED;
			std::vector<int32_t> mob_targets;
			std::vector<int32_t> reactor_targets = find_targets(reactor_objs, range, origin, mobcount, false, true);
			bool shot = attack.type == Attack::Type::RANGED && whack == Stance::Id::NONE && mobcount == 1;
			bool spell = attack.type == Attack::Type::MAGIC && shot_path::aimed_spell(attack.skill);

			if (spell)
				reach = shot_path::SPELL_REACH;

			Point<int16_t> shot_start = shot_path::origin(origin, reach, attack.toleft);
			Point<int16_t> shot_end(attack.toleft ? range.left() : range.right(), shot_start.y());

			// A shot at one mob, or an aimed spell, hits the first in its path
			// instead. A platform between the player and it stops a shot, or a
			// spell that flies.
			if (shot || spell)
			{
				int16_t shot_range = spell ? shot_path::SPELL_RANGE : get_shot_range(attack);
				shot_end.set_x(origin.x() + (attack.toleft ? -shot_range : shot_range));
				mob_targets = find_shot_target(shot_start, shot_end.x());

				Point<int16_t> sight = shot_path::origin(origin, 0, attack.toleft);
				Point<int16_t> stop = shot_end;

				// Chain Lightning strikes its first mob without flying there,
				// and jumps on from it.
				if (chain_lightning::chains(attack.skill))
				{
					if (!mob_targets.empty())
						mob_targets = find_chain(mob_targets[0], mobcount, attack.toleft);
				} else
				{
					if (!mob_targets.empty())
						stop = shot_path::hit_point(shot_start, shot_end.x(), mobs.get_mob_body(mob_targets[0]));

					if (attack.bullet && shot_path::blocked(sight, stop, find_platforms(sight, stop), player.get_phobj().fhgroup))
					{
						mob_targets.clear();
						shot_end = stop;
					}
				}
			} else
			{
				mob_targets = find_targets(mob_objs, range, origin, mobcount, true, closest_first);
			}

			mobs.send_attack(result, attack, mob_targets, mobcount);
			result.attacker = player.get_oid();
			expect_hits(result, 1);
			extract_effects(player, move, result, range, reach, shot_end);

			apply_use_movement(move);
			apply_result_movement(move, result);

			AttackPacket(result).dispatch();

			if (reactor_targets.size())
				if (Optional<Reactor> reactor = reactor_objs->get(reactor_targets.at(0)))
					DamageReactorPacket(reactor->get_oid(), player.get_position(), 0, 0).dispatch();
		} else
		{
			request_skill(move);
		}
	}

	Stance::Id Combat::find_whack(const Attack &attack, int16_t &reach) const
	{
		Stance::Id stance = player.get_look().get_attackstance(true);
		Rectangle<int16_t> range = point_blank::range(player.get_afterimage_range(stance));
		reach = -range.left();
		range = place_range(attack.origin, range, 1.0f, attack.toleft);

		if (find_targets(mobs.get_mobs(), range, attack.origin, 1, true, true).empty())
			return Stance::Id::NONE;

		return stance;
	}

	int16_t Combat::get_whack_reach(const Char &user, Stance::Id stance)
	{
		return -point_blank::range(user.get_afterimage_range(stance)).left();
	}

	int16_t Combat::get_shot_range(const Attack &attack) const
	{
		Weapon::Type weapon = player.get_stats().get_weapontype();
		int32_t range_skill = shot_path::range_skill(weapon, player.get_stats().get_job().get_id());
		int16_t bonus = 0;

		if (int32_t level = player.get_skills().get_level(range_skill))
			bonus = SkillData::get(range_skill).get_stats(level).shotrange;

		return shot_path::range(attack.skill, attack.shotrange, weapon, bonus);
	}

	std::vector<int32_t> Combat::find_shot_target(Point<int16_t> origin, int16_t end_x) const
	{
		MapObjects *objs = mobs.get_mobs();

		for (Rectangle<int16_t> step : shot_path::search_steps(player.get_position().x(), origin, end_x))
		{
			// The mobs in one step are tried in the order they entered the map.
			int32_t target = 0;
			uint32_t first = UINT32_MAX;

			for (auto &mmo : *objs)
			{
				const Mob *mob = static_cast<const Mob *>(mmo.second.get());

				if (mob && mob->is_alive() && shot_path::in_step(step, mob->get_body()))
				{
					uint32_t entry = objs->entry_order(mob->get_oid());

					if (entry < first)
					{
						first = entry;
						target = mob->get_oid();
					}
				}
			}

			if (first != UINT32_MAX)
				return { target };
		}

		return {};
	}

	std::vector<int32_t> Combat::find_chain(int32_t first, uint8_t count, bool toleft) const
	{
		MapObjects *objs = mobs.get_mobs();
		std::vector<chain_lightning::Mob> alive;
		std::optional<chain_lightning::Mob> start;

		for (auto &mmo : *objs)
		{
			const Mob *mob = static_cast<const Mob *>(mmo.second.get());

			if (!mob || !mob->is_alive())
				continue;

			chain_lightning::Mob candidate = {
				mob->get_oid(), mob->get_position(), mob->get_body(), objs->entry_order(mob->get_oid())
			};

			if (candidate.oid == first)
				start = candidate;

			alive.push_back(candidate);
		}

		if (!start)
			return {};

		return chain_lightning::chain(*start, alive, count, toleft);
	}

	std::vector<teleport::Floor> Combat::find_floors() const
	{
		const FootholdTree &fht = physics.get_fht();
		std::vector<teleport::Floor> floors;

		for (uint16_t group : fht.get_groups())
		{
			for (uint16_t fhid : fht.get_group_footholds(group))
			{
				const Foothold &fh = fht.get_fh(fhid);

				floors.push_back({ fhid, { fh.x1(), fh.y1(), fh.x2(), fh.y2() } });
			}
		}

		return floors;
	}

	std::vector<shot_path::Platform> Combat::find_platforms(Point<int16_t> from, Point<int16_t> to) const
	{
		const FootholdTree &fht = physics.get_fht();
		Range<int16_t> across(std::min(from.x(), to.x()), std::max(from.x(), to.x()));
		Range<int16_t> down(std::min(from.y(), to.y()), std::max(from.y(), to.y()));

		auto line = [&](uint16_t fhid) -> std::optional<shot_path::Line>
		{
			const Foothold &fh = fht.get_fh(fhid);

			if (fhid == 0 || fh.id() != fhid)
				return std::nullopt;

			return shot_path::Line{ fh.x1(), fh.y1(), fh.x2(), fh.y2() };
		};

		std::vector<shot_path::Platform> platforms;

		for (uint16_t group : fht.get_groups())
		{
			for (uint16_t fhid : fht.get_group_footholds(group))
			{
				const Foothold &fh = fht.get_fh(fhid);

				// v83 asks its foothold tree for those whose bounds touch the
				// shot's (FUN_00508740).
				if (fh.l() > across.second() || fh.r() < across.first() || fh.t() > down.second() || fh.b() < down.first())
					continue;

				platforms.push_back({ *line(fhid), group, line(fh.prev()), line(fh.next()) });
			}
		}

		return platforms;
	}

	std::vector<int32_t>
	Combat::find_targets(MapObjects *objs, Rectangle<int16_t> range, Point<int16_t> origin, uint8_t objcount,
						 bool use_mobs, bool closest_first) const
	{
		std::multimap<uint32_t, int32_t> candidates;

		for (auto &mmo : *objs)
		{
			if (use_mobs)
			{
				const Mob *mob = static_cast<const Mob *>(mmo.second.get());

				if (mob && mob->is_alive() && mob->is_in_range(range))
				{
					int32_t oid = mob->get_oid();
					uint32_t key = closest_first ? mob->get_position().distance(origin) : objs->entry_order(oid);
					candidates.emplace(key, oid);
				}
			} else
			{
				// Assume Reactor
				const Reactor *reactor = static_cast<const Reactor *>(mmo.second.get());

				if (reactor && reactor->is_hittable() && reactor->is_in_range(range))
				{
					int32_t oid = reactor->get_oid();
					uint32_t key = closest_first ? reactor->get_position().distance(origin) : objs->entry_order(oid);
					candidates.emplace(key, oid);
				}
			}
		}

		std::vector<int32_t> targets;

		for (auto &iter : candidates)
		{
			if (targets.size() >= objcount)
				break;

			targets.push_back(iter.second);
		}


		return targets;
	}

	void Combat::apply_use_movement(const SpecialMove &move)
	{
		switch (move.get_id())
		{
			case SkillId::Id::TELEPORT_FP:
			case SkillId::Id::IL_TELEPORT:
			case SkillId::Id::PRIEST_TELEPORT:
			case SkillId::Id::FLASH_JUMP:
			default:
				break;
		}
	}

	void Combat::apply_result_movement(const SpecialMove &move, const AttackResult &result)
	{
		switch (move.get_id())
		{
			case SkillId::Id::RUSH_HERO:
			case SkillId::Id::RUSH_PALADIN:
			case SkillId::Id::RUSH_DK:
				apply_rush(result);
				break;
			default:
				break;
		}
	}

	void Combat::apply_rush(const AttackResult &result)
	{
		if (result.mobcount == 0)
			return;

		Point<int16_t> mob_position = mobs.get_mob_position(result.last_oid);
		int16_t targetx = mob_position.x();
		player.rush(targetx);
	}

	void Combat::apply_bullet_effect(const Bullet &bullet)
	{
		bullets.push_back(bullet);
	}

	void Combat::apply_damage_effect(const DamageEffect &effect)
	{
		Point<int16_t> head_position = mobs.get_mob_head_position(effect.target_oid);
		damagenumbers.push_back(effect.number);
		damagenumbers.back().set_anchor(head_position);

		const SpecialMove &move = get_move(effect.move_id);
		mobs.apply_damage(effect.target_oid, effect.damage, effect.toleft, effect.user, move);
	}

	void Combat::push_attack(const AttackResult &attack)
	{
		expect_hits(attack, 1);
		attackresults.push(400, attack);
	}

	void Combat::expect_hits(const AttackResult &result, int32_t sign)
	{
		for (auto &line : result.damagelines)
			mobs.expect_hits(line.first, sign * static_cast<int32_t>(line.second.size()));
	}

	void Combat::apply_attack(const AttackResult &attack)
	{
		if (Optional<OtherChar> ouser = chars.get_char(attack.attacker))
		{
			OtherChar &user = *ouser;
			user.update_skill(attack.skill, attack.level);
			user.update_speed(attack.speed);

			const SpecialMove &move = get_move(attack.skill);
			move.apply_useeffects(user);

			if (Stance::Id stance = Stance::by_id(attack.stance))
				user.attack(stance);
			else
				move.apply_actions(user, attack.type);

			user.set_afterimage(attack.skill);

			Rectangle<int16_t> range = place_range(user.get_position(), user.get_afterimage().get_range(), 1.0f,
												   attack.toleft);
			// Other players' shots start where a whack would reach, as the
			// player's own do.
			int16_t reach = 0;

			bool spell = attack.type == Attack::Type::MAGIC && shot_path::aimed_spell(attack.skill);

			if (attack.type == Attack::Type::RANGED && attack.skill == 0)
				reach = get_whack_reach(user, user.get_look().get_attackstance(true));
			else if (spell)
				reach = shot_path::SPELL_REACH;

			// Their range skills' levels are unknown, so their shots reach
			// only as far as the weapon or skill alone does.
			int16_t skill_range = attack.skill ? SkillData::get(attack.skill).get_stats(attack.level).shotrange : 0;
			int16_t shot_range = spell ? shot_path::SPELL_RANGE
				: shot_path::range(attack.skill, skill_range, user.get_weapontype(), 0);
			Point<int16_t> shot_end = shot_path::origin(user.get_position(), shot_range, attack.toleft);
			extract_effects(user, move, attack, range, reach, shot_end);
		} else
		{
			expect_hits(attack, -1);
		}
	}

	Rectangle<int16_t> Combat::place_range(Point<int16_t> origin, Rectangle<int16_t> range, float hrange, bool toleft)
	{
		int16_t reach = static_cast<int16_t>(range.left() * hrange);

		if (toleft)
		{
			return {
					static_cast<int16_t>(origin.x() + reach),
					static_cast<int16_t>(origin.x() + range.right()),
					static_cast<int16_t>(origin.y() + range.top()),
					static_cast<int16_t>(origin.y() + range.bottom())
			};
		} else
		{
			return {
					static_cast<int16_t>(origin.x() - range.right()),
					static_cast<int16_t>(origin.x() - reach),
					static_cast<int16_t>(origin.y() + range.top()),
					static_cast<int16_t>(origin.y() + range.bottom())
			};
		}
	}

	void Combat::extract_effects(const Char &user, const SpecialMove &move, const AttackResult &result,
								 Rectangle<int16_t> range, int16_t reach, Point<int16_t> shot_end)
	{
		AttackUser attackuser = {
				user.get_skilllevel(move.get_id()),
				user.get_level(),
				user.is_twohanded(),
				!result.toleft,
				user.get_look().get_equips().get_weapon(),
				result.stance,
				range
		};

		bool chain = chain_lightning::chains(result.skill);

		if (chain)
			draw_chain(user, move, result);

		if (result.bullet && !chain)
		{
			// As in v83, the bullets fly straight from in front of the user to
			// where the shot crosses each target, or as far as the shot goes.
			// A target's hits all land 1.5 ms per pixel it is away across, or
			// along a spell's path, after the attack's hit frame. A spell
			// flies as one ball however many times it hits.
			Animation animation = move.get_bullet(user, result.bullet);
			Point<int16_t> origin = shot_path::origin(user.get_position(), reach, result.toleft);
			uint16_t delay = user.get_attackdelay(0);
			bool spell = result.type == Attack::Type::MAGIC;

			auto shoot = [&](Point<int16_t> target, uint8_t count)
			{
				if (spell)
					count = 1;

				for (uint8_t i = 0; i < count; i++)
				{
					Point<int16_t> end = target + Point<int16_t>(0, shot_path::spread(i, count));
					uint16_t start = delay + shot_path::bullet_delay(result.bullet, result.skill, i);
					bulleteffects.push(start, Bullet(animation, origin, end, shot_path::flight_time(origin, end)));
				}
			};

			for (auto &line : result.damagelines)
			{
				int32_t oid = line.first;

				if (mobs.contains(oid))
				{
					std::vector<DamageNumber> numbers = place_numbers(line.second, move.get_id());
					Point<int16_t> hit = shot_path::hit_point(origin, shot_end.x(), mobs.get_mob_body(oid));
					uint16_t hit_delay = delay + (spell ? shot_path::flight_time(origin, hit) : shot_path::hit_delay(origin, hit));

					shoot(hit, static_cast<uint8_t>(numbers.size()));

					size_t i = 0;

					for (auto &number : numbers)
					{
						damageeffects.emplace(
								hit_delay,
								attackuser,
								number,
								line.second[i].first,
								result.toleft,
								oid,
								move.get_id()
						);

						i++;
					}
				}
			}

			if (result.damagelines.empty())
				shoot(shot_end, result.hitcount);
		} else
		{
			// As in v83, every hit on a mob lands at once, at the frame the
			// attack hits on, and each later target a little after.
			uint16_t delay = user.get_attackdelay(0);

			for (size_t target = 0; target < result.damagelines.size(); target++)
			{
				auto &line = result.damagelines[target];
				int32_t oid = line.first;

				if (mobs.contains(oid))
				{
					std::vector<DamageNumber> numbers = place_numbers(line.second, move.get_id());
					uint16_t target_delay = hit_timing::target_delay(result.type, result.skill, target);

					size_t i = 0;

					for (auto &number : numbers)
					{
						damageeffects.emplace(
								delay + target_delay,
								attackuser,
								number,
								line.second[i].first,
								result.toleft,
								oid,
								move.get_id()
						);

						i++;
					}
				}
			}
		}
	}

	void Combat::draw_chain(const Char &user, const SpecialMove &move, const AttackResult &result)
	{
		// The bolt jumps from the user to each mob in turn as it is hit,
		// one ball after another along each jump, every one of the three
		// kinds of ball at random.
		Randomizer random;
		Point<int16_t> from = chain_lightning::start(user.get_position(), result.toleft);
		uint16_t delay = user.get_attackdelay(0);

		for (size_t target = 0; target < result.damagelines.size(); target++)
		{
			int32_t oid = result.damagelines[target].first;

			if (!mobs.contains(oid))
				continue;

			Point<int16_t> to = chain_lightning::middle(mobs.get_mob_body(oid));
			uint16_t start = delay + hit_timing::target_delay(result.type, result.skill, target);

			for (chain_lightning::Ball ball : chain_lightning::bolt(from, to))
			{
				Animation animation = move.get_ball(random.next_int(chain_lightning::BALL_KINDS));
				float angle = static_cast<float>(ball.degrees * M_PI / 180.0);
				bulleteffects.push(start, Bullet(animation, ball.position, ball.position, chain_lightning::BOLT_TIME, angle));
			}

			from = to;
		}
	}

	std::vector<DamageNumber>
	Combat::place_numbers(const std::vector<std::pair<int32_t, bool>> &damagelines, int32_t move_id)
	{
		// Demolition's many hits stack half as far apart.
		bool close = move_id == 5121004;
		std::vector<DamageNumber> numbers;

		for (size_t line = 0; line < damagelines.size(); line++)
		{
			int32_t amount = damagelines[line].first;
			bool critical = damagelines[line].second;
			DamageNumber::Type type = critical ? DamageNumber::Type::CRITICAL : DamageNumber::Type::NORMAL;
			int16_t lift = damage_layout::mob_offset(static_cast<int16_t>(line), close);
			numbers.emplace_back(type, amount, lift);
		}

		return numbers;
	}

	void Combat::show_buff(int32_t cid, int32_t skillid, int8_t level)
	{
		if (Optional<OtherChar> ouser = chars.get_char(cid))
		{
			OtherChar &user = *ouser;
			user.update_skill(skillid, level);

			const SpecialMove &move = get_move(skillid);
			move.apply_useeffects(user);
			move.apply_actions(user, Attack::Type::MAGIC);
		}
	}

	void Combat::show_player_buff(int32_t skillid)
	{
		get_move(skillid).apply_useeffects(player);
	}

	void Combat::start_prepare(const Skill &skill)
	{
		int32_t id = skill.get_id();

		// Like a skill that doesn't attack, a held one waits until the
		// player may ask the server, but holds nothing up itself (0x485bf7).
		// It starts only from a platform.
		if (!player.can_request(ExclRequest::SKILL_WAIT) || !player.get_phobj().onground || player.get_ladder())
			return;

		// Chakra says nothing when the player has too much HP.
		const CharStats &stats = player.get_stats();

		if (id == prepare::CHAKRA
			&& !prepare::chakra_allowed(stats.get_stat(MapleStat::Id::HP), stats.get_total(EquipStat::Id::HP)))
			return;

		const SkillPrepare &data = skill.get_prepare();
		int16_t action = prepare::action_id(data.action);
		int32_t level = player.get_skills().get_level(id);
		bool forced = action >= 0;
		bool timed = prepare::timed(id);

		// The skills that keep firing take up their action and stay on one
		// frame of it; the timed ones play it once. A `dash` is no action:
		// the player keeps walking.
		if (forced)
		{
			player.attack(data.action);

			if (!timed)
				player.hold_attack(prepare::held_frame(data.action));
		}

		player.clear_held_effects();
		player.show_held_effect(data.prepare.animation, data.prepare.z, false);

		// The skills that keep firing, and the timed ones, sound as they're
		// used, the others over and over while they charge (0x9692d1).
		if (prepare::repeats(id) || timed)
			skill.play_use();
		else
			skill.play_loop();

		uint16_t action_and_facing = prepare::action_and_facing(action, !player.getflip());
		PrepareSkillPacket(id, level, action_and_facing, player.get_action_speed()).dispatch();

		int32_t duration = timed && forced ? player.get_action_duration(data.action) : 0;
		preparing = Preparing{ id, level, 0, forced, false, duration };
	}

	void Combat::update_prepare()
	{
		if (!preparing)
			return;

		preparing->held += Constants::TIMESTEP;

		if (prepare::timed(preparing->skill))
		{
			if (preparing->held >= preparing->duration)
				complete_prepare();

			return;
		}

		const Skill &skill = get_skill(preparing->skill);
		const SkillPrepare &data = skill.get_prepare();

		// The prepare effect gives way to the keydown one, played over and
		// over, once the skill's time has passed (0x96cf9f).
		if (!preparing->keydown && preparing->held >= data.time)
		{
			preparing->keydown = true;
			player.clear_held_effects();
			player.show_held_effect(data.keydown.animation, data.keydown.z, true);
		}

		if (prepare::repeats(preparing->skill))
			shoot_prepare(skill);
	}

	void Combat::shoot_prepare(const Skill &skill)
	{
		int32_t id = skill.get_id();
		int64_t &last = id == prepare::RAPID_FIRE ? last_rapidfire : last_hurricane;

		if (!prepare::shot_due(now, last))
			return;

		last = now;

		// Each shot needs its arrows or bullets, and the MP, or the skill is
		// let go (0x94bae5).
		SpecialMove::ForbidReason reason = player.can_use(skill);

		if (reason != SpecialMove::ForbidReason::FBR_NONE)
		{
			ForbidSkillMessage(reason, player.get_stats().get_weapontype()).drop();
			finish_prepare();
			return;
		}

		// Hurricane sounds with each arrow, Rapid Fire only as it starts
		// (0x94bbed).
		if (id != prepare::RAPID_FIRE)
			skill.play_use();

		apply_move(skill, 0, true);
	}

	void Combat::complete_prepare()
	{
		int32_t id = preparing->skill;
		const Skill &skill = get_skill(id);

		// The action is over; its effect plays out by itself.
		if (preparing->forced && player.is_attacking())
			player.end_attack();

		preparing.reset();

		if (!player.can_attack())
			return;

		if (id == prepare::EXPLOSION)
			apply_move(skill);
		else
			request_skill(skill);
	}

	void Combat::release_prepare(int32_t move_id)
	{
		if (preparing && preparing->skill == move_id)
			finish_prepare();
	}

	void Combat::interrupt_prepare(int32_t damage)
	{
		if (preparing && damage > 0 && prepare::repeats(preparing->skill))
			finish_prepare();
	}

	void Combat::clear_prepare()
	{
		if (!preparing)
			return;

		get_skill(preparing->skill).stop_loop();
		player.clear_held_effects();

		if (preparing->forced)
			player.end_attack();

		preparing.reset();
	}

	void Combat::finish_prepare()
	{
		// A timed skill isn't let go: it goes off when its time is up.
		if (!preparing || prepare::timed(preparing->skill))
			return;

		Preparing held = *preparing;
		const Skill &skill = get_skill(held.skill);

		clear_prepare();
		player.show_held_effect(skill.get_prepare().keydownend.animation, skill.get_prepare().keydownend.z, false);

		// The skills that keep firing just stop, and say so.
		if (prepare::repeats(held.skill))
		{
			CancelBuffPacket(held.skill).dispatch();
			return;
		}

		// Piercing Arrow whacks a mob right in front of the player instead
		// (0x95c090).
		int16_t reach = 0;

		if (held.skill == prepare::PIERCING_ARROW && can_use_move(regularattack)
			&& find_whack(player.prepare_attack(0), reach) != Stance::Id::NONE)
		{
			apply_move(regularattack);
			return;
		}

		if (can_use_move(skill))
			apply_move(skill, prepare::release_charge(held.skill, held.held));
		else if (held.skill != prepare::CORKSCREW_BLOW && held.skill != prepare::CORKSCREW_BLOW_TB)
			CancelBuffPacket(held.skill).dispatch();
	}

	const Skill &Combat::get_skill(int32_t skill_id)
	{
		return static_cast<const Skill &>(get_move(skill_id));
	}

	const SpecialMove &Combat::get_move(int32_t move_id)
	{
		if (move_id == 0)
			return regularattack;

		auto iter = skills.find(move_id);

		if (iter == skills.end())
			iter = skills.emplace(move_id, move_id).first;

		return iter->second;
	}
}
