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
#include "Heal.h"
#include "HitTiming.h"
#include "MagicDamage.h"
#include "PointBlank.h"
#include "Prepare.h"
#include "RemoteShot.h"
#include "RemoteSpell.h"
#include "ShotPath.h"

#include "../MapleMap/DoorRules.h"

#include "../../Constants.h"
#include "../../Character/SkillId.h"
#include "../../Data/SkillData.h"
#include "../../IO/Messages.h"
#include "../../Util/Misc.h"
#include "../../Util/Randomizer.h"

#include "../../Net/Packets/AttackAndSkillPackets.h"
#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cmath>
#include <cstdlib>

namespace ms
{
	Combat::Combat(Player &in_player, MapChars &in_chars, MapMobs &in_mobs, MapReactors &in_reactors,
				   MapSummons &in_summons, const Physics &in_physics, const MapInfo &in_mapinfo,
				   const MapPortals &in_portals) : player(
			in_player), chars(in_chars), mobs(in_mobs), reactors(in_reactors), summons(in_summons),
			physics(in_physics), mapinfo(in_mapinfo), portals(in_portals),
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
																												}),
																										summonhits(
																												[&](const SummonHit &hit)
																												{
																													apply_summon_hit(hit);
																												}),
																										magnetcatches(
																												[&](const MagnetCatch &caught)
																												{
																													// Its hit lands: the mob shows
																													// whether it was caught, and is
																													// pulled in if so (0x668e14).
																													show_catch(caught.oid, caught.success);

																													if (caught.success)
																														mobs.catch_mob(caught.oid, caught.user);
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

		// Monster Magnet's catch, and its miss: shown, then faded out, 200 ms
		// each (0x438eb6).
		nl::node catcheffect = nl::nx::effect["BasicEff.img"]["Catch"];
		catch_success = catcheffect["Success"];
		catch_fail = std::vector<Frame>{ Frame(catcheffect["Fail"], 200, 255, 255), Frame(catcheffect["Fail"], 200, 255, 0) };

		// Poison Bomb throws Grenade's grenade too (0x58f761).
		grenade_ball = nl::nx::skill["520.img"]["skill"]["5201002"]["ball"];
	}

	Texture Combat::gauge_bar[2];
	Texture Combat::gauge_fill[2];
	Texture Combat::gauge_graduation[2];
	Animation Combat::catch_success;
	Animation Combat::catch_fail;
	Animation Combat::grenade_ball;

	void Combat::draw(double viewx, double viewy, float alpha) const
	{
		draw_rains(viewx, viewy, alpha);

		for (auto &bullet : bullets)
			bullet.draw(viewx, viewy, alpha);

		for (auto &grenade : grenades)
			grenade.ball.draw(DrawArgument(grenade.phobj.get_absolute(viewx, viewy, alpha)), alpha);

		for (auto &dn : damagenumbers)
			dn.draw(viewx, viewy, alpha);

		fieldeffects.drawabove(Point<int16_t>(static_cast<int16_t>(viewx), static_cast<int16_t>(viewy)), alpha);

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
		update_remote_prepares();
		update_grenades();
		update_rains();
		update_summons();

		attackresults.update();
		bulleteffects.update();
		damageeffects.update();
		summonhits.update();
		magnetcatches.update();
		fieldeffects.update();

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

		// Poison and the like show as a plain first hit, a heal in blue
		// (0x6691d3).
		for (const auto &[head, number] : mobs.take_hurts())
		{
			DamageNumber::Type type = number < 0 ? DamageNumber::Type::HEAL : DamageNumber::Type::NORMAL;
			damagenumbers.emplace_back(type, std::abs(number), damage_layout::mob_offset(0, false));
			damagenumbers.back().set_anchor(head);
		}
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

		int32_t moveid = move.get_id();
		int32_t level = player.get_skills().get_level(moveid);

		if (summon_rules::is_summon(moveid))
			return request_summon(move, level);

		if (moveid == door_rules::MYSTIC_DOOR)
			return request_door(move, level);

		std::optional<UseSkillPacket::Reach> reach = find_reach(moveid, level);

		if (!reach)
			return false;

		move.apply_useeffects(player);
		std::string action = move.apply_actions(player, Attack::Type::MAGIC);
		player.fix_action_speed(6);

		// Resurrection with no one to raise plays out but isn't sent
		// (0x96d567).
		if (!skill_use::sends(moveid, reach->party))
			return true;

		uint16_t lock = action.empty() ? 0 : player.get_action_lock(action);
		player.request(UseSkillPacket(moveid, level, lock, *reach), ExclRequest::SKILL_WAIT);

		return true;
	}

	bool Combat::request_summon(const SpecialMove &move, int32_t level)
	{
		int32_t moveid = move.get_id();
		bool left = !player.getflip();
		Point<int16_t> position = player.get_position();
		Range<int16_t> walls = mapinfo.get_walls();
		auto ground = [&](int16_t x, int16_t top)
		{
			return physics.get_fht().get_y_below(Point<int16_t>(x, top));
		};
		auto spot = summon_rules::place(moveid, position.x(), position.y(), left, walls.first(), walls.second(),
										 ground);

		// v83 says nothing when there's no room for it.
		if (!spot)
			return false;

		move.apply_useeffects(player);
		move.apply_actions(player, Attack::Type::MAGIC);
		player.fix_action_speed(6);
		player.request(SummonSkillPacket(moveid, level, Point<int16_t>(spot->first, spot->second), left),
					   ExclRequest::SKILL_WAIT);

		return true;
	}

	bool Combat::request_door(const SpecialMove &move, int32_t level)
	{
		Point<int16_t> position = player.get_position();
		door_rules::Cast cast = door_rules::may_cast(mapinfo.get_fieldlimit(), player.get_stats().get_mapid(),
													 mapinfo.is_town(), player.get_phobj().onground,
													 portals.blocks_door(position));

		switch (cast)
		{
			case door_rules::Cast::FORBIDDEN:
				InChatMessage(Messages::SKILL_FIELD).drop();
				return false;
			case door_rules::Cast::NEAR_PORTAL:
				InChatMessage(Messages::DOOR_PORTAL).drop();
				return false;
			case door_rules::Cast::SILENT:
				return false;
			case door_rules::Cast::OPEN:
				break;
		}

		move.apply_useeffects(player);
		move.apply_actions(player, Attack::Type::MAGIC);
		player.fix_action_speed(6);
		player.request(DoorSkillPacket(move.get_id(), level, position), ExclRequest::SKILL_WAIT);

		return true;
	}

	const Combat::SummonInfo &Combat::get_summon_info(int32_t skill)
	{
		auto iter = summoninfos.find(skill);

		if (iter != summoninfos.end())
			return iter->second;

		nl::node src = nl::nx::skill[string_format::extend_id(skill / 10000, 3) + ".img"]["skill"]
		[string_format::extend_id(skill, 7)];
		nl::node info = src["summon"]["attack1"]["info"];
		nl::node range = info["range"];
		nl::node hit = info["hit"] ? info["hit"] : src["hit"]["0"];

		SummonInfo summon{
				static_cast<uint8_t>(info["mobCount"] ? info["mobCount"].get_integer() : 1),
				static_cast<int16_t>(range["r"].get_integer()),
				range["lt"] ? Rectangle<int16_t>(range["lt"], range["rb"]) : Rectangle<int16_t>(),
				static_cast<int16_t>(info["attackAfter"].get_integer()),
				static_cast<int16_t>(info["ball"] ? info["bulletSpeed"].get_integer() : 0),
				Animation(hit)
		};

		return summoninfos.emplace(skill, summon).first->second;
	}

	void Combat::update_summons()
	{
		// A mob that touches the player's Puppet hurts it; the server breaks
		// it at 0 HP (FUN_007a3f4d, FUN_007a6040).
		for (auto &[oid, summon] : summons.get_local())
		{
			if (!summon->can_be_hit())
				continue;

			int32_t mob = mobs.find_touching(summon->get_body());

			if (!mob)
				continue;

			int32_t damage = mobs.roll_puppet_damage(mob, summon->get_level());
			bool left = mobs.get_mob_position(mob).x() < summon->get_position().x();
			summons.hurt(oid, damage);
			DamageSummonPacket(oid, damage, mobs.get_mob_id(mob), left).dispatch();
		}

		if (player.is_dead())
			return;

		// A Beholder heals its owner when it can, else buffs them, each in
		// its skill's time (FUN_007a4080).
		for (auto &[oid, summon] : summons.get_local())
		{
			if (summon->get_assist() != summon_rules::SUPPORT)
				continue;

			if (summon->next_heal < 0)
			{
				summon->next_heal = now + summon_rules::SUPPORT_DELAY;
				summon->next_hex = now + summon_rules::SUPPORT_DELAY;
			}

			if (!summon->is_idle())
				continue;

			if (now > summon->next_heal && beholder_heal(oid, *summon))
				continue;

			if (now > summon->next_hex)
				beholder_hex(oid, *summon);
		}

		for (auto &[oid, summon] : summons.get_local())
		{
			int32_t skill = summon->get_skill();

			if (summon->get_assist() != summon_rules::ATTACK || !summon->is_idle() ||
				now - summon->last_attack < summon_rules::attack_interval(skill))
				continue;

			const SummonInfo &info = get_summon_info(skill);
			Point<int16_t> position = summon->get_position();
			MapObjects *objs = mobs.get_mobs();
			std::vector<int32_t> targets;
			bool left = summon->is_left();

			if (summon_rules::aims_at_box(skill))
			{
				// The mage summons hit the side of their box with more mobs
				// (FUN_00765047).
				Rectangle<int16_t> area = info.area;
				Rectangle<int16_t> facing_left(position + area.get_left_top(), position + area.get_right_bottom());
				Rectangle<int16_t> facing_right(position.x() - area.right(), position.x() - area.left(),
												position.y() + area.top(), position.y() + area.bottom());
				std::vector<int32_t> lefts = find_targets(objs, facing_left, position, info.mobcount, true, false);
				std::vector<int32_t> rights = find_targets(objs, facing_right, position, info.mobcount, true, false);
				left = lefts.size() >= rights.size();
				targets = left ? lefts : rights;
			} else
			{
				// The others hit the mobs nearest them, within their reach
				// (I: within 100 px up or down).
				int16_t reach = info.reach > 0 ? info.reach : 300;
				Rectangle<int16_t> around(position.x() - reach, position.x() + reach, position.y() - 100,
										  position.y() + 100);
				uint8_t count = summon_rules::spreads(skill) ? 1 : info.mobcount;
				targets = find_targets(objs, around, position, count, true, true);

				if (!targets.empty() && summon_rules::spreads(skill))
				{
					Point<int16_t> first = mobs.get_mob_position(targets[0]);
					Rectangle<int16_t> spread(first + info.area.get_left_top(), first + info.area.get_right_bottom());

					for (int32_t other : find_targets(objs, spread, first, info.mobcount, true, true))
						if (other != targets[0] && targets.size() < info.mobcount)
							targets.push_back(other);
				}

				if (!targets.empty())
					left = mobs.get_mob_position(targets[0]).x() < position.x();
			}

			if (targets.empty())
				continue;

			nl::node level = nl::nx::skill[string_format::extend_id(skill / 10000, 3) + ".img"]["skill"]
			[string_format::extend_id(skill, 7)]["level"][std::to_string(summon->get_level())];
			const CharStats &stats = player.get_stats();
			int32_t str = stats.get_total(EquipStat::Id::STR);
			int32_t dex = stats.get_total(EquipStat::Id::DEX);
			int32_t intl = stats.get_total(EquipStat::Id::INT);
			int32_t luk = stats.get_total(EquipStat::Id::LUK);
			int32_t main = str;

			switch (summon_rules::main_stat(stats.get_job().get_id()))
			{
			case summon_rules::DEX:
				main = dex;
				break;
			case summon_rules::LUK:
				main = luk;
				break;
			default:
				break;
			}

			Randomizer random;
			uint8_t action = skill == summon_rules::WRATH_OF_THE_OCTOPI && random.next_bool() ? summon_rules::ATTACK2
																								  : summon_rules::ATTACK1;
			std::vector<SummonAttackPacket::Hit> hits;

			for (size_t i = 0; i < targets.size(); i++)
			{
				int32_t target = targets[i];
				double roll = random.next_real(1.0);
				int32_t damage;

				if (summon_rules::is_magic(skill))
				{
					int32_t magic = magic_damage::magic(intl, stats.get_total(EquipStat::Id::MAGIC));
					damage = std::max(1, static_cast<int32_t>(
							magic_damage::base(magic, intl, 0, level["mad"].get_integer(), roll)));
				} else
				{
					damage = summon_rules::physical_damage(main, str + dex + luk - main,
														   static_cast<int32_t>(level["pad"].get_integer()), roll);
				}

				Point<int16_t> at = mobs.get_mob_position(target);
				int16_t delay = info.attackafter;

				if (info.bulletspeed > 0)
					delay += static_cast<int16_t>(at.distance(position) * 1000 / info.bulletspeed);

				if (summon_rules::spreads(skill))
					delay += static_cast<int16_t>(120 * i);

				hits.push_back({ target, mobs.get_mob_id(target), at, at.x() > position.x(), delay, damage });
				summonhits.push(delay, { target, damage, left, skill });
			}

			summon->set_left(left);
			summon->play(static_cast<summon_rules::Action>(action));
			summon->last_attack = now;

			SummonAttackPacket(oid, action, left, player.get_position(), position, hits).dispatch();
		}
	}

	namespace
	{
		nl::node skill_level(int32_t skill, int32_t level)
		{
			return nl::nx::skill[string_format::extend_id(skill / 10000, 3) + ".img"]["skill"]
			[string_format::extend_id(skill, 7)]["level"][std::to_string(level)];
		}
	}

	bool Combat::beholder_heal(int32_t oid, Summon &beholder)
	{
		SkillBook &skills = player.get_skills();
		int32_t level = skills.get_level(summon_rules::AURA_OF_THE_BEHOLDER);

		if (level <= 0)
			return false;

		nl::node aura = skill_level(summon_rules::AURA_OF_THE_BEHOLDER, level);
		int32_t berserk_level = skills.get_level(SkillId::Id::BERSERK);
		int32_t berserk = berserk_level > 0
						  ? static_cast<int32_t>(skill_level(SkillId::Id::BERSERK, berserk_level)["x"].get_integer())
						  : 0;
		const CharStats &stats = player.get_stats();

		if (!summon_rules::aura_heals(stats.get_stat(MapleStat::Id::HP), stats.get_total(EquipStat::Id::HP),
									  static_cast<int32_t>(aura["hp"].get_integer()), berserk))
			return false;

		BeholderPacket(oid, beholder.is_left(), -1).dispatch();
		beholder.next_heal = now + aura["x"].get_integer() * 1000;
		beholder.play(summon_rules::SKILL1);

		return true;
	}

	void Combat::beholder_hex(int32_t oid, Summon &beholder)
	{
		int32_t level = player.get_skills().get_level(summon_rules::HEX_OF_THE_BEHOLDER);

		if (level <= 0)
			return;

		nl::node hex = skill_level(summon_rules::HEX_OF_THE_BEHOLDER, level);
		std::array<int32_t, 5> gives = {
				static_cast<int32_t>(hex["pdd"].get_integer()), static_cast<int32_t>(hex["mdd"].get_integer()),
				static_cast<int32_t>(hex["acc"].get_integer()), static_cast<int32_t>(hex["eva"].get_integer()),
				static_cast<int32_t>(hex["pad"].get_integer())
		};
		std::array<int32_t, 5> has = {
				player.get_buff_value(Buffstat::Id::WDEF), player.get_buff_value(Buffstat::Id::MDEF),
				player.get_buff_value(Buffstat::Id::ACC), player.get_buff_value(Buffstat::Id::AVOID),
				player.get_buff_value(Buffstat::Id::WATK)
		};
		Randomizer random;
		int8_t hexed = summon_rules::hex_pick(gives, has, random.next_int<uint32_t>(UINT32_MAX));

		if (hexed < 0)
			return;

		BeholderPacket(oid, beholder.is_left(), hexed).dispatch();
		beholder.next_hex = now + hex["x"].get_integer() * 1000;
		beholder.play(static_cast<summon_rules::Action>(summon_rules::SKILL1 + 1 + hexed));
	}

	void Combat::apply_summon_hit(const SummonHit &hit)
	{
		if (!mobs.contains(hit.oid))
			return;

		damagenumbers.emplace_back(DamageNumber::Type::NORMAL, hit.damage, damage_layout::mob_offset(0, false));
		damagenumbers.back().set_anchor(mobs.get_mob_head_position(hit.oid));
		mobs.apply_summon_damage(hit.oid, hit.damage, hit.toleft, get_summon_info(hit.skill).hit);
	}

	void Combat::show_summon_attack(int32_t oid, uint8_t action, const std::vector<std::pair<int32_t, int32_t>> &hits)
	{
		Summon *summon = summons.get(oid);

		if (!summon)
			return;

		bool left = action & 0x80;
		int32_t skill = summon->get_skill();
		const SummonInfo &info = get_summon_info(skill);
		Point<int16_t> position = summon->get_position();
		summon->play(static_cast<summon_rules::Action>(action & 0x7F));

		for (size_t i = 0; i < hits.size(); i++)
		{
			int32_t delay = info.attackafter;

			if (info.bulletspeed > 0)
				delay += static_cast<int32_t>(mobs.get_mob_position(hits[i].first).distance(position) * 1000 /
											  info.bulletspeed);

			if (summon_rules::spreads(skill))
				delay += static_cast<int32_t>(120 * i);

			summonhits.push(delay, { hits[i].first, hits[i].second, left, skill });
		}
	}

	std::optional<UseSkillPacket::Reach> Combat::find_reach(int32_t skill, int32_t level)
	{
		UseSkillPacket::Reach reach;
		const SkillData::Stats &stats = SkillData::get(skill).get_stats(level);
		Point<int16_t> position = player.get_position();

		if (skill_use::sends_position(skill))
			reach.position = position;

		if (skill == skill_use::SHADOW_STARS)
		{
			if (player.get_stats().get_weapontype() == Weapon::CLAW)
			{
				const Inventory &inventory = player.get_inventory();
				std::vector<std::pair<int32_t, int32_t>> slots;

				for (int16_t slot = 1; slot <= inventory.get_slotmax(InventoryType::Id::USE); slot++)
					slots.emplace_back(inventory.get_item_id(InventoryType::Id::USE, slot),
									   inventory.get_item_count(InventoryType::Id::USE, slot));

				reach.star = skill_use::shadow_star(slots, stats.bulletcost);
			}

			if (reach.star == 0)
			{
				InChatMessage(Messages::SKILL_NOSTARS).drop();
				return std::nullopt;
			}
		}

		// The party members and mobs in the skill's area, in front of the
		// player (0x96db3f, 0x96dd2b).
		Rectangle<int16_t> range = place_range(position, stats.range, 1.0f, !player.getflip());
		uint8_t kinds = skill_use::reach(skill);
		int32_t members = 0;

		if (kinds & skill_use::PARTY)
			members = find_party(skill, range, reach.party);

		if (kinds & skill_use::MOBS)
			reach.mobs = find_targets(mobs.get_mobs(), range, position, skill_use::mob_limit(stats.mobcount, members),
									  true, false);

		return reach;
	}

	int32_t Combat::find_party(int32_t skill, Rectangle<int16_t> range, uint8_t &mask) const
	{
		const Party &party = player.get_party();
		int32_t members = 0;

		if (!party.exists())
		{
			mask = skill_use::ALONE;
			members = 1;
		}

		for (const PartyMember &member : party.get_members())
		{
			bool self = member.id == player.get_oid();
			bool dead = false;

			if (self)
			{
				dead = player.is_dead();
			}
			else
			{
				Optional<OtherChar> other = chars.get_char(member.id);

				if (!other || !range.contains(other->get_position()))
					continue;

				dead = other->is_dead();
			}

			if (skill_use::reaches(skill, self, dead))
			{
				mask |= skill_use::party_bit(member.slot);
				members++;
			}
		}

		return members;
	}

	int32_t Combat::request_heal()
	{
		int32_t level = player.get_skills().get_level(heal::ID);
		const SkillData::Stats &stats = SkillData::get(heal::ID).get_stats(level);
		Rectangle<int16_t> range = place_range(player.get_position(), stats.range, 1.0f, !player.getflip());

		UseSkillPacket::Reach reach;
		int32_t members = find_party(heal::ID, range, reach.party);

		player.request(UseSkillPacket(heal::ID, level, 0, reach), ExclRequest::SKILL_WAIT);

		return members;
	}

	std::vector<int32_t> Combat::find_undead(Rectangle<int16_t> range, uint8_t count) const
	{
		MapObjects *objs = mobs.get_mobs();
		std::multimap<uint32_t, int32_t> candidates;

		for (auto &mmo : *objs)
		{
			const Mob *mob = static_cast<const Mob *>(mmo.second.get());

			if (mob && mob->is_alive() && mob->is_undead() && mob->is_in_range(range))
				candidates.emplace(objs->entry_order(mob->get_oid()), mob->get_oid());
		}

		std::vector<int32_t> targets;

		for (auto &candidate : candidates)
		{
			if (targets.size() >= count)
				break;

			targets.push_back(candidate.second);
		}

		return targets;
	}

	void Combat::apply_move(const SpecialMove &move, int32_t charge, bool held)
	{
		if (move.is_attack())
		{
			// Heal asks the server first, as a skill that doesn't attack.
			int32_t healed = 0;

			if (move.get_id() == heal::ID)
			{
				if (!player.can_request(ExclRequest::SKILL_WAIT))
					return;

				healed = request_heal();
			}

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

			// Shadow Partner throws each bullet again, for twice the hits.
			if (attack.type == Attack::Type::RANGED && player.has_buff(Buffstat::Id::SHADOWPARTNER) &&
				shot_path::partner_shoots(attack.skill))
			{
				attack.sets = 2;
				attack.hitcount = shot_path::partner_hits(attack.hitcount);
			}

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
			bool shot = attack.type == Attack::Type::RANGED && whack == Stance::Id::NONE;
			bool spell = attack.type == Attack::Type::MAGIC && shot_path::aimed_spell(attack.skill);

			if (spell)
				reach = shot_path::SPELL_REACH;

			Point<int16_t> shot_start = shot_path::origin(origin, reach, attack.toleft);
			Point<int16_t> shot_end(attack.toleft ? range.left() : range.right(), shot_start.y());
			Rectangle<int16_t> shot_area;

			// As in v83, a shot or an aimed spell hits the first mob in its
			// path, and a platform between the player and it stops it, or a
			// spell that flies. Some shots hit the first mobs to enter the
			// map in an area instead: their row widened, or the skill's own
			// area (FUN_009537d5).
			if (shot || spell)
			{
				int16_t shot_range = spell ? shot_path::SPELL_RANGE : get_shot_range(attack);
				shot_end.set_x(origin.x() + (attack.toleft ? -shot_range : shot_range));
				shot_area = shot_path::area(attack.skill, origin, shot_start, shot_end.x(), attack.range, attack.toleft);

				if (shot && shot_path::area_shot(attack.skill))
				{
					mob_targets = find_targets(mob_objs, shot_area, origin, mobcount, true, false);
				} else
				{
					mob_targets = find_shot_target(shot_start, shot_end.x());

					Point<int16_t> sight = shot_path::origin(origin, 0, attack.toleft);
					Point<int16_t> stop = shot_end;

					// Chain Lightning strikes its first mob without flying
					// there, and jumps on from it.
					if (chain_lightning::chains(attack.skill))
					{
						if (!mob_targets.empty())
							mob_targets = find_chain(mob_targets[0], mobcount, attack.toleft);
					} else
					{
						if (!mob_targets.empty())
							stop = shot_path::hit_point(shot_area, mobs.get_mob_body(mob_targets[0]));

						if (attack.bullet && shot_path::blocked(sight, stop, find_platforms(sight, stop), player.get_phobj().fhgroup))
						{
							mob_targets.clear();
							shot_end = stop;
						}
					}

					if (!mob_targets.empty() && mobcount > 1)
					{
						// Energy Orb jumps on from the first mob as Chain
						// Lightning does, and Arrow Bomb, Inferno and Blizzard
						// hit the mobs around it too.
						if (shot_path::chains(attack.skill))
						{
							mob_targets = find_chain(mob_targets[0], mobcount, attack.toleft);
						} else if (shot_path::splashes(attack.skill))
						{
							int32_t first = mob_targets[0];
							Point<int16_t> hit = shot_path::hit_point(shot_area, mobs.get_mob_body(first));
							Rectangle<int16_t> splash = shot_path::splash_area(hit, attack.range);

							for (int32_t oid : find_targets(mob_objs, splash, origin, mobcount, true, false))
								if (oid != first && mob_targets.size() < mobcount)
									mob_targets.push_back(oid);
						}
					}
				}
			} else if (attack.skill == heal::ID)
			{
				// It hurts fewer mobs the more of the party it heals, and each
				// less the more it reaches.
				range = place_range(origin, heal::area(attack.range), 1.0f, attack.toleft);
				mob_targets = find_undead(range, skill_use::mob_limit(mobcount, healed));
				attack.healed = healed + static_cast<int32_t>(mob_targets.size());
			} else
			{
				mob_targets = find_targets(mob_objs, range, origin, mobcount, true, closest_first);
			}

			mobs.send_attack(result, attack, mob_targets, mobcount);

			if (!result.damagelines.empty() && !result.damagelines[0].second.empty()
				&& shot_path::drops_splash(attack.skill, result.damagelines[0].second[0].first))
			{
				result.damagelines.resize(1);
				result.mobcount = 1;
				result.last_oid = result.first_oid;
			}

			result.attacker = player.get_oid();
			expect_hits(result, 1);
			extract_effects(player, move, result, range, reach, shot_area, shot_end);

			if (attack.type == Attack::Type::MAGIC)
				cast_area(attack.skill, attack.range, attack.charge, player.get_position(), attack.toleft);

			apply_use_movement(move);
			apply_result_movement(move, result);

			AttackPacket(result, player.get_position()).dispatch();

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

			// An attack ends the skill the user was preparing, unless it
			// is one of its shots (0x98044f).
			if (!prepare::outlasts_attacks(attack.skill))
				end_remote_prepare(user);

			const SpecialMove &move = get_move(attack.skill);

			// A grenade going off moves no one, and has its own effect
			// (0x980712).
			if (!prepare::throws(attack.skill))
			{
				move.apply_useeffects(user);

				if (Stance::Id stance = Stance::by_id(attack.stance))
					user.attack(stance);
				else
					move.apply_actions(user, attack.type);
			}

			user.set_afterimage(attack.skill);

			Rectangle<int16_t> range = place_range(user.get_position(), user.get_afterimage().get_range(), 1.0f,
												   attack.toleft);
			// Their shots and spells have their own paths; anything else that
			// flies starts right at them.
			int16_t reach = 0;

			// Their range skills' levels are unknown, so their shots reach
			// only as far as the weapon or skill alone does.
			Rectangle<int16_t> skill_area;
			int16_t skill_range = 0;

			if (attack.skill)
			{
				const SkillData::Stats &stats = SkillData::get(attack.skill).get_stats(attack.level);
				skill_area = stats.range;
				skill_range = stats.shotrange;
			}

			int16_t shot_range = shot_path::range(attack.skill, skill_range, user.get_weapontype(), 0);
			Point<int16_t> shot_end = shot_path::origin(user.get_position(), shot_range, attack.toleft);
			Rectangle<int16_t> shot_area = shot_path::area(attack.skill, user.get_position(),
														   shot_path::origin(user.get_position(), reach, attack.toleft),
														   shot_end.x(), skill_area, attack.toleft);

			if (attack.type == Attack::Type::MAGIC)
			{
				show_remote_spell(user, move, attack, range);
				cast_area(attack.skill, skill_area, attack.charge, user.get_position(), attack.toleft);
			}
			else if (attack.type == Attack::Type::RANGED && !prepare::throws(attack.skill))
				show_remote_shot(user, move, attack, range);
			else
				extract_effects(user, move, attack, range, reach, shot_area, shot_end);
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
								 Rectangle<int16_t> range, int16_t reach, Rectangle<int16_t> shot_area,
								 Point<int16_t> shot_end)
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
			draw_chain(user, move, result, true);

		if (result.bullet && !chain)
		{
			// As in v83, a shot's bullets fly once, straight from in front of
			// the user to where it crosses the first mob it hits in its path,
			// or as far as it goes for a shot at an area or one that hit
			// nothing. A spell flies one ball to each mob it hits. A mob's
			// hits all land 1.5 ms per pixel it is away across, or along a
			// spell's path, after the attack's hit frame, but for a few
			// skills (0x954867).
			Animation animation = move.get_bullet(user, result.bullet);
			Point<int16_t> origin = shot_path::origin(user.get_position(), reach, result.toleft);
			Point<int16_t> start = shot_path::bullet_start(result.skill, origin, result.toleft);
			uint16_t delay = user.get_attackdelay(0);
			bool spell = result.type == Attack::Type::MAGIC;

			// Shadow Partner's set follows the user's, from behind them, each
			// bullet further below the last.
			auto shoot = [&](Point<int16_t> target, uint8_t count)
			{
				uint8_t sets = result.sets;

				if (spell)
					count = sets = 1;
				else if (shot_path::hides_bullet(result.skill))
					return;

				uint8_t per_set = std::max(count / sets, 1);

				for (uint8_t i = 0; i < count; i++)
				{
					Point<int16_t> from = i < per_set ? start : shot_path::partner_start(start, result.toleft);
					Point<int16_t> end = target + Point<int16_t>(0, shot_path::spread(i, per_set));
					uint16_t time = delay + shot_path::bullet_delay(result.bullet, result.skill, i);
					bulleteffects.push(time, Bullet(animation, from, end, shot_path::flight_time(from, end)));
				}
			};

			uint16_t first = 0;
			std::optional<Point<int16_t>> first_hit;
			std::vector<Rectangle<int16_t>> bodies;

			for (size_t target = 0; target < result.damagelines.size(); target++)
			{
				auto &line = result.damagelines[target];
				int32_t oid = line.first;

				if (mobs.contains(oid))
				{
					std::vector<DamageNumber> numbers = place_numbers(line.second, move.get_id());
					Rectangle<int16_t> body = mobs.get_mob_body(oid);
					Point<int16_t> hit = shot_path::hit_point(shot_area, body);
					uint16_t hit_delay = spell ? shot_path::flight_time(origin, hit)
						: shot_path::target_delay(result.skill, target, first, origin, hit);

					if (target == 0)
						first = hit_delay;

					if (!first_hit)
						first_hit = hit;

					bodies.push_back(body);

					if (spell)
						shoot(hit, 1);
					else if (target == 0 && !shot_path::area_shot(result.skill))
						shoot(hit, result.hitcount);

					size_t i = 0;

					for (auto &number : numbers)
					{
						damageeffects.emplace(
								delay + hit_delay,
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

			if (result.damagelines.empty() || (!spell && shot_path::area_shot(result.skill)))
				shoot(shot_end, result.hitcount);

			if (!spell && first_hit && shot_rain::kind(result.skill) != shot_rain::Kind::NONE)
				rain_shot(result.skill, SkillData::get(result.skill).get_stats(attackuser.skilllevel).range,
						  user.get_position(), result.toleft, delay + first, *first_hit, bodies);
		} else
		{
			// As in v83, every hit on a mob lands at once, at the frame the
			// attack hits on, and each later target a little after. A
			// grenade's land as it goes off.
			uint16_t delay = prepare::throws(result.skill) ? 0 : user.get_attackdelay(0);

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

	void Combat::show_remote_spell(const Char &user, const SpecialMove &move, const AttackResult &result,
								   Rectangle<int16_t> range)
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

		int32_t skill = result.skill;
		bool ball = move.has_ball();
		bool balls = ball && !remote_spell::one_ball(skill) && !remote_spell::bolts(skill);
		Animation animation = ball ? move.get_bullet(user, skill) : Animation();
		Point<int16_t> position = user.get_position();
		uint16_t delay = user.get_attackdelay(0);
		uint16_t hits_delay = remote_spell::attack_delay(skill, delay);

		if (chain_lightning::chains(skill))
			draw_chain(user, move, result, false);

		for (size_t target = 0; target < result.damagelines.size(); target++)
		{
			auto &line = result.damagelines[target];
			int32_t oid = line.first;

			if (!mobs.contains(oid))
				continue;

			Rectangle<int16_t> body = mobs.get_mob_body(oid);
			bool left = mobs.get_mob_position(oid).x() < position.x();
			Point<int16_t> start = remote_spell::start(position, left);

			// A mob that reaches back past where the balls start ends the
			// spell there: neither it nor the mobs after it show their hits.
			if (remote_spell::needs_ahead(skill) && !remote_spell::ahead(start.x(), body, left))
			{
				for (size_t rest = target; rest < result.damagelines.size(); rest++)
				{
					auto &skipped = result.damagelines[rest];
					mobs.expect_hits(skipped.first, -static_cast<int32_t>(skipped.second.size()));
				}

				break;
			}

			Point<int16_t> aim = remote_spell::aim(start, body, left);
			uint16_t flight = remote_spell::hit_delay(skill, target, ball, start, aim);
			std::vector<DamageNumber> numbers = place_numbers(line.second, move.get_id());
			size_t shown = remote_spell::shown_hits(skill, numbers.size());

			for (size_t i = 0; i < shown; i++)
				damageeffects.emplace(
						hits_delay + flight,
						attackuser,
						numbers[i],
						line.second[i].first,
						result.toleft,
						oid,
						move.get_id()
				);

			if (shown < numbers.size())
				mobs.expect_hits(oid, -static_cast<int32_t>(numbers.size() - shown));

			if (balls)
				bulleteffects.push(delay, Bullet(animation, start, remote_spell::end(start, aim), flight));
		}

		if (!ball || remote_spell::bolts(skill))
			return;

		Point<int16_t> start = remote_spell::start(position, result.toleft);

		if (result.damagelines.empty())
		{
			// A spell that hit nothing flies out to 200 px in front of them.
			int16_t miss = result.toleft ? -remote_spell::MISS_RANGE : remote_spell::MISS_RANGE;
			Point<int16_t> end(static_cast<int16_t>(position.x() + miss), start.y());
			bulleteffects.push(delay, Bullet(animation, start, end, remote_spell::across_time(start.x(), end.x())));
		} else if (remote_spell::one_ball(skill))
		{
			// Its one ball flies straight across to the furthest mob it hit.
			std::optional<int16_t> furthest;

			for (auto &line : result.damagelines)
			{
				if (!mobs.contains(line.first))
					continue;

				int16_t x = mobs.get_mob_position(line.first).x();

				if (!furthest || (result.toleft ? x < *furthest : *furthest < x))
					furthest = x;
			}

			if (furthest && (result.toleft ? *furthest < start.x() : start.x() < *furthest))
			{
				Point<int16_t> end(*furthest, start.y());
				bulleteffects.push(delay, Bullet(animation, start, end, remote_spell::across_time(start.x(), end.x())));
			}
		}
	}

	void Combat::show_remote_shot(const OtherChar &user, const SpecialMove &move, const AttackResult &result,
								  Rectangle<int16_t> range)
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

		int32_t skill = result.skill;
		uint8_t bullets = 1;
		uint8_t attacks = 1;
		Rectangle<int16_t> lt_rb;

		if (skill)
		{
			const SkillData::Stats &stats = SkillData::get(skill).get_stats(result.level);
			bullets = std::max<uint8_t>(stats.bulletcount, 1);
			attacks = stats.attackcount;
			lt_rb = stats.range;
		}

		// Shadow Partner throws every bullet twice (0x981447).
		uint8_t sets = user.get_buff(Buffstat::Id::SHADOWPARTNER) ? 2 : 1;
		Animation animation = move.get_bullet(user, result.bullet);
		Point<int16_t> position = user.get_position();
		uint16_t delay = remote_shot::attack_delay(skill, user.get_attackdelay(0));
		uint16_t first = 0;
		std::optional<Point<int16_t>> first_hit;
		std::vector<Rectangle<int16_t>> bodies;

		for (size_t target = 0; target < result.damagelines.size(); target++)
		{
			auto &line = result.damagelines[target];
			int32_t oid = line.first;

			if (!mobs.contains(oid))
				continue;

			Rectangle<int16_t> body = mobs.get_mob_body(oid);

			// A mob with no bounds ends the shot there: neither it nor the
			// mobs after it show their hits.
			if (body.left() >= body.right() || body.top() >= body.bottom())
			{
				for (size_t rest = target; rest < result.damagelines.size(); rest++)
				{
					auto &skipped = result.damagelines[rest];
					mobs.expect_hits(skipped.first, -static_cast<int32_t>(skipped.second.size()));
				}

				break;
			}

			bool left = mobs.get_mob_position(oid).x() < position.x();
			Point<int16_t> start = remote_shot::start(skill, position, left, result.toleft);
			Point<int16_t> aim = remote_spell::aim(start, body, left);
			uint16_t hit = remote_shot::hit_delay(skill, target, first, delay, start, aim);

			if (target == 0)
				first = hit;

			if (!first_hit)
				first_hit = remote_shot::end(start, aim);

			bodies.push_back(body);

			std::vector<DamageNumber> numbers = place_numbers(line.second, move.get_id());
			size_t shown = remote_shot::shown_hits(numbers.size(), sets * bullets, attacks);

			for (size_t i = 0; i < shown; i++)
				damageeffects.emplace(
						delay + hit,
						attackuser,
						numbers[i],
						line.second[i].first,
						result.toleft,
						oid,
						move.get_id()
				);

			if (shown < numbers.size())
				mobs.expect_hits(oid, -static_cast<int32_t>(numbers.size() - shown));

			if (!remote_shot::shoots(skill, target, lt_rb, start.x(), body, left))
				continue;

			// Each bullet flies as long as the hits take to land, to at most
			// 400 px along the way, each a few px below the last.
			Point<int16_t> end = remote_shot::end(start, aim);
			Point<int16_t> from = remote_shot::partner_starts(skill, target, bullets, sets)
				? shot_path::partner_start(start, result.toleft) : start;

			for (uint8_t i = 0; i < sets * bullets; i++)
			{
				Point<int16_t> to = end + Point<int16_t>(0, shot_path::spread(i, bullets));
				uint16_t time = delay + shot_path::bullet_delay(result.bullet, skill, i);
				bulleteffects.push(time, Bullet(animation, from, to, hit));
			}
		}

		if (first_hit)
			rain_shot(skill, lt_rb, position, result.toleft, delay + first, *first_hit, bodies);

		if (!result.damagelines.empty() || remote_shot::hides_miss(skill))
			return;

		// A shot that hit nothing flies out in front of them.
		Weapon::Type weapon = user.get_weapontype();
		int16_t reach = remote_shot::miss_range(weapon == Weapon::Type::BOW || weapon == Weapon::Type::CROSSBOW);
		int16_t y = position.y() - shot_path::HEIGHT;
		Point<int16_t> start(remote_shot::miss_start(skill, position.x(), result.toleft), y);
		int16_t end_x = position.x() + (result.toleft ? -reach : reach);
		uint16_t flight = remote_shot::miss_flight(skill, start.x(), end_x);

		// With Shadow Partner they all stop 30 px short, as fast.
		if (sets > 1)
			end_x = shot_path::partner_start(Point<int16_t>(end_x, y), result.toleft).x();

		for (uint8_t i = 0; i < sets * bullets; i++)
		{
			Point<int16_t> end(end_x, static_cast<int16_t>(y + remote_shot::miss_spread(i, bullets)));
			uint16_t time = delay + shot_path::bullet_delay(result.bullet, skill, i);
			bulleteffects.push(time, Bullet(animation, start, end, flight));
		}
	}

	void Combat::rain_shot(int32_t skill, Rectangle<int16_t> lt_rb, Point<int16_t> user, bool toleft, int64_t hit,
						   Point<int16_t> first, const std::vector<Rectangle<int16_t>> &bodies)
	{
		switch (shot_rain::kind(skill))
		{
			case shot_rain::Kind::OVER_USER:
				start_rain(get_skill(skill), shot_rain::over_user(lt_rb, user, toleft), toleft, hit);
				break;
			case shot_rain::Kind::OVER_HIT:
			{
				Rectangle<int16_t> area = shot_rain::around(lt_rb, first);

				if (shot_rain::burns(skill))
					burn(get_skill(skill), area, hit, shot_rain::BURN);

				start_rain(get_skill(skill), shot_rain::raised(area), toleft, hit);
				break;
			}
			case shot_rain::Kind::OVER_MOBS:
				for (Rectangle<int16_t> body : bodies)
					start_rain(get_skill(skill), shot_rain::over_mob(body), toleft, hit);

				break;
			default:
				break;
		}
	}

	void Combat::start_rain(const Skill &skill, Rectangle<int16_t> area, bool toleft, int64_t hit)
	{
		if (skill.get_rain().drops.empty())
			return;

		rains.push_back({ skill.get_id(), area, toleft, shot_rain::Schedule(skill.get_rain().params, now + hit) });
	}

	void Combat::burn(const Skill &skill, Rectangle<int16_t> area, int64_t hit, int32_t length)
	{
		const SkillRain &rain = skill.get_rain();

		if (rain.flames.empty())
			return;

		Randomizer randomizer;
		const FootholdTree &fht = physics.get_fht();

		for (int16_t x : shot_rain::columns(area, rain.distance))
		{
			for (uint16_t group : fht.get_groups())
			{
				for (uint16_t id : fht.get_group_footholds(group))
				{
					const Foothold &fh = fht.get_fh(id);
					shot_path::Line platform{ fh.x1(), fh.y1(), fh.x2(), fh.y2() };
					std::optional<int16_t> ground = shot_rain::ground(platform, x, area);

					if (!ground)
						continue;

					flames.push_back({
							rain.flames[randomizer.next_int<size_t>(rain.flames.size())],
							{ x, shot_rain::flame_y(*ground, randomizer.next_int(5)) },
							now + hit,
							shot_rain::flame_fade(randomizer.next_int(5)),
							(128 + randomizer.next_int(128)) / 255.0f,
							length
					});
				}
			}
		}
	}

	void Combat::cast_area(int32_t skill, Rectangle<int16_t> lt_rb, int32_t charge, Point<int16_t> user, bool toleft)
	{
		spell_area::Kind kind = spell_area::kind(skill);

		if (kind == spell_area::Kind::NONE)
			return;

		const Skill &area_skill = get_skill(skill);

		if (kind == spell_area::Kind::TILES)
		{
			burn(area_skill, spell_area::tiles(lt_rb, user, toleft), spell_area::TILE_DELAY, spell_area::burn_time(skill));

			return;
		}

		if (area_skill.get_rain().drops.empty())
			return;

		Rectangle<int16_t> area = kind == spell_area::Kind::BIG_BANG
			? spell_area::big_bang(charge, user)
			: spell_area::explosion(lt_rb, user);

		blasts.push_back({ skill, shot_rain::Schedule(area_skill.get_rain().params, now), spell_area::Spread(area) });
	}

	void Combat::update_rains()
	{
		Randomizer randomizer;

		// Each wave of a blast plays once where it lands (0x441f45).
		for (Blast &blast : blasts)
		{
			if (!blast.schedule.drop(now))
				continue;

			const SkillRain &skillrain = get_skill(blast.skill).get_rain();
			Rectangle<int16_t> band = blast.spread.widen();

			for (int32_t i = 0; i < skillrain.params.count; i++)
			{
				Point<int16_t> position(
						randomizer.next_int(band.left(), band.right()),
						randomizer.next_int(band.top(), band.bottom())
				);

				fieldeffects.add(skillrain.drops[randomizer.next_int<size_t>(skillrain.drops.size())],
								 DrawArgument(position), 0);
			}
		}

		blasts.remove_if(
				[&](const Blast &blast)
				{
					return blast.schedule.over(now);
				}
		);

		for (Rain &rain : rains)
		{
			if (!rain.schedule.drop(now))
				continue;

			const SkillRain &skillrain = get_skill(rain.skill).get_rain();
			const shot_rain::Params &params = skillrain.params;

			for (int32_t i = 0; i < params.count; i++)
			{
				Point<int16_t> start(
						randomizer.next_int(rain.area.left(), rain.area.right()),
						randomizer.next_int(rain.area.top(), rain.area.bottom())
				);
				int32_t flight = shot_rain::flight(params, randomizer.next_int(300));

				if (flight <= 0)
					continue;

				raindrops.push_back({
						skillrain.drops[randomizer.next_int<size_t>(skillrain.drops.size())],
						start,
						shot_rain::fall(params, rain.toleft, randomizer.next_int(41)),
						!rain.toleft,
						now,
						flight,
						shot_rain::opacity(randomizer.next_int(64))
				});
			}
		}

		rains.remove_if(
				[&](const Rain &rain)
				{
					return rain.schedule.over(now);
				}
		);

		raindrops.remove_if(
				[&](RainDrop &drop)
				{
					drop.animation.update();

					return now >= drop.launch + drop.flight;
				}
		);

		flames.remove_if(
				[&](Flame &flame)
				{
					if (now > flame.start)
						flame.animation.update();

					return now > flame.start + flame.length + shot_rain::BURN;
				}
		);
	}

	void Combat::draw_rains(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> view(static_cast<int16_t>(viewx), static_cast<int16_t>(viewy));

		// The fire fades in once lit, burns, then fades out (0x442887).
		for (const Flame &flame : flames)
		{
			int64_t out = flame.start + flame.length;
			float opacity = now <= out
				? shot_rain::fade(0.0f, flame.opacity, flame.start, flame.start + flame.fade, now)
				: shot_rain::fade(flame.opacity, 0.0f, out, out + shot_rain::BURN, now);

			if (opacity > 0.0f)
				flame.animation.draw(DrawArgument(flame.position + view, opacity), alpha);
		}

		// A drop moves evenly along its fall, fading from opaque (0x441b30).
		for (const RainDrop &drop : raindrops)
		{
			float progress = static_cast<float>(now - drop.launch) / drop.flight;
			Point<int16_t> position = drop.start + Point<int16_t>(
					static_cast<int16_t>(drop.fall.x() * progress),
					static_cast<int16_t>(drop.fall.y() * progress)
			);
			float opacity = shot_rain::fade(1.0f, drop.opacity, drop.launch, drop.launch + shot_rain::FADE, now);

			drop.animation.draw(DrawArgument(position + view, drop.flip, opacity), alpha);
		}
	}

	void Combat::draw_chain(const Char &user, const SpecialMove &move, const AttackResult &result, bool staggered)
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
			uint16_t start = delay;

			if (staggered)
				start += hit_timing::target_delay(result.type, result.skill, target);

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

	void Combat::show_affected(int32_t cid, int32_t skillid)
	{
		if (Optional<OtherChar> ouser = chars.get_char(cid))
		{
			const SkillPrepare::Effect &affected = get_skill(skillid).get_affected();
			ouser->show_attack_effect(affected.animation, affected.z);
		}
	}

	void Combat::show_player_affected(int32_t skillid)
	{
		const SkillPrepare::Effect &affected = get_skill(skillid).get_affected();
		player.show_attack_effect(affected.animation, affected.z);
	}

	void Combat::show_prepare(int32_t cid, int32_t skillid, uint8_t level, uint16_t action_and_facing, uint8_t speed)
	{
		Optional<OtherChar> ouser = chars.get_char(cid);

		if (!ouser || skillid <= 0)
			return;

		OtherChar &user = *ouser;
		end_remote_prepare(user);

		const Skill &skill = get_skill(skillid);
		const SkillPrepare &data = skill.get_prepare();
		bool keydown = prepare::keydown(skillid);
		int16_t action = prepare::prepared_action(action_and_facing);
		std::string name = prepare::action_name(action);

		user.update_skill(skillid, std::max<uint8_t>(level, 1));
		user.update_speed(speed);

		// They face the way they prepare it, and take up its action: held
		// on one frame for a held skill, played once for a timed one. No
		// action leaves them as they are.
		if (action != 0x7fff)
			user.set_direction(!prepare::prepared_toleft(action_and_facing));

		if (!name.empty())
		{
			user.attack(name);

			if (keydown)
				user.hold_attack(prepare::held_frame(name));
		}

		user.clear_held_effects();
		user.show_held_effect(data.prepare.animation, data.prepare.z, false);

		// Only a held skill is followed until it's let go.
		if (keydown)
			remote_prepares[cid] = RemotePrepare{ skillid, 0, false, !name.empty() };
	}

	void Combat::cancel_prepare(int32_t cid, int32_t skillid)
	{
		Optional<OtherChar> ouser = chars.get_char(cid);

		if (!ouser)
			return;

		OtherChar &user = *ouser;
		end_remote_prepare(user);

		// The skills that keep firing wind down; the effect of any other
		// just goes.
		if (prepare::repeats(skillid))
		{
			const SkillPrepare &data = get_skill(skillid).get_prepare();
			user.show_held_effect(data.keydownend.animation, data.keydownend.z, false);
		}
	}

	void Combat::update_remote_prepares()
	{
		for (auto iter = remote_prepares.begin(); iter != remote_prepares.end();)
		{
			Optional<OtherChar> ouser = chars.get_char(iter->first);

			if (!ouser)
			{
				iter = remote_prepares.erase(iter);
				continue;
			}

			RemotePrepare &held = iter->second;
			held.held += Constants::TIMESTEP;

			const SkillPrepare &data = get_skill(held.skill).get_prepare();

			if (!held.keydown && held.held >= data.time)
			{
				held.keydown = true;
				ouser->clear_held_effects();
				ouser->show_held_effect(data.keydown.animation, data.keydown.z, true);
			}

			++iter;
		}
	}

	void Combat::end_remote_prepare(OtherChar &user)
	{
		user.clear_held_effects();

		auto iter = remote_prepares.find(user.get_oid());

		if (iter == remote_prepares.end())
			return;

		if (iter->second.forced)
			user.end_attack();

		remote_prepares.erase(iter);
	}

	void Combat::start_prepare(const Skill &skill)
	{
		int32_t id = skill.get_id();

		// Like a skill that doesn't attack, a held one waits until the
		// player may ask the server, but holds nothing up itself (0x485bf7).
		// It starts only from a platform, but a grenade may be thrown from
		// the air.
		bool thrown = prepare::throws(id);

		if (!player.can_request(ExclRequest::SKILL_WAIT) || (!thrown && !player.get_phobj().onground)
			|| player.get_ladder())
			return;

		// Monster Magnet and the throws wait a while to be held again
		// (0x96a9ab).
		int32_t reuse = prepare::reuse_delay(id);

		if (reuse > 0)
		{
			auto iter = reuse_times.find(id);

			if (iter != reuse_times.end() && now < iter->second)
				return;
		}

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

		if (reuse > 0)
			reuse_times[id] = now + reuse;
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

		// A grenade is thrown, unless the player is on a ladder, and hits
		// where it lands (0x95c13b).
		if (prepare::throws(held.skill))
		{
			if (!player.get_ladder())
			{
				int32_t charge = prepare::release_charge(held.skill, held.held);
				Point<int16_t> position = player.get_position();

				throw_grenade(player.get_oid(), held.skill, charge, position, !player.getflip());
				ThrowGrenadePacket(position, charge, held.skill).dispatch();
			}

			return;
		}

		if (magnet::is_magnet(held.skill))
		{
			if (!use_magnet(skill, held.level, prepare::release_charge(held.skill, held.held)))
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

	bool Combat::use_magnet(const Skill &skill, int32_t level, int32_t charge)
	{
		// It isn't used where the map forbids movement skills, nor without
		// leave to ask the server (0x94be52).
		if (!mapinfo.can_use_movement_skills() || !player.can_request(ExclRequest::SKILL_WAIT) || !can_use_move(skill))
			return false;

		int32_t id = skill.get_id();
		const SkillData::Stats &stats = SkillData::get(id).get_stats(level);
		int32_t count = magnet::count(stats.mobcount, charge, prepare::charge_cap(id));

		if (count > magnet::MAX_COUNT)
			return false;

		// It looks for mobs from 50 px in front of the player to its range,
		// taking them in the order they entered the map (0x678aec).
		bool toleft = !player.getflip();
		Point<int16_t> position = player.get_position();
		Point<int16_t> origin = shot_path::origin(position, magnet::START, toleft);
		int16_t end_x = position.x() + (toleft ? -stats.shotrange : stats.shotrange);
		std::vector<Rectangle<int16_t>> steps = shot_path::search_steps(position.x(), origin, end_x);

		MapObjects *objs = mobs.get_mobs();
		std::vector<std::pair<uint32_t, const Mob *>> reached;

		for (auto &mmo : *objs)
		{
			const Mob *mob = static_cast<const Mob *>(mmo.second.get());

			if (!mob || !mob->is_alive())
				continue;

			Rectangle<int16_t> body = mob->get_body();

			for (Rectangle<int16_t> step : steps)
			{
				if (shot_path::in_step(step, body))
				{
					reached.emplace_back(objs->entry_order(mob->get_oid()), mob);
					break;
				}
			}
		}

		std::sort(reached.begin(), reached.end(), [](auto &a, auto &b) { return a.first < b.first; });

		if (reached.size() > static_cast<size_t>(count))
			reached.resize(count);

		// A mob out of the player's sight is sent as 0. The rest are caught
		// at the skill's prop (0x96c3c0).
		Point<int16_t> sight = { position.x(), origin.y() };
		uint16_t group = player.get_phobj().fhgroup;
		int32_t prop = static_cast<int32_t>(std::lround(stats.chance * 100));
		Randomizer random;

		std::vector<std::pair<int32_t, bool>> found;
		std::vector<MagnetCatch> caught;

		for (auto &[entry, mob] : reached)
		{
			Point<int16_t> hit = shot_path::hit_point(origin, end_x, mob->get_body());

			if (shot_path::blocked(sight, hit, find_platforms(sight, hit), group))
			{
				found.emplace_back(0, false);
				continue;
			}

			bool success = magnet::catches(mob->is_boss(), mob->can_move(), prop, random.next_int(100));

			found.emplace_back(mob->get_oid(), success);
			caught.push_back({ mob->get_oid(), success, position });
		}

		player.request(MonsterMagnetPacket(id, level, found, toleft), ExclRequest::SKILL_WAIT);
		skill.apply_useeffects(player);

		for (const MagnetCatch &mob : caught)
			magnetcatches.push(magnet::HIT_DELAY, mob);

		return true;
	}

	void Combat::show_catch(int32_t oid, bool success)
	{
		if (!mobs.contains(oid))
			return;

		Point<int16_t> position = mobs.get_mob_position(oid) + Point<int16_t>(0, -15);

		fieldeffects.add(success ? catch_success : catch_fail, DrawArgument(position), 0);
	}

	void Combat::show_throw(int32_t cid, Point<int16_t> position, int32_t charge, int32_t skillid)
	{
		if (Optional<OtherChar> user = chars.get_char(cid))
			throw_grenade(cid, skillid, charge, position, !user->getflip());
	}

	void Combat::throw_grenade(int32_t owner, int32_t skill, int32_t charge, Point<int16_t> position, bool toleft)
	{
		// It flies up and away from the user as fast as it was held long,
		// as a knockback would push it, and falls as anything does.
		int32_t speed = prepare::throw_speed(charge);
		Point<int16_t> start = prepare::throw_start(position, toleft);

		Grenade grenade{ owner, skill, {}, grenade_ball };
		grenade.phobj.shoe = true;
		grenade.phobj.onground = false;
		grenade.phobj.set_x(start.x());
		grenade.phobj.set_y(start.y());
		grenade.phobj.impact = true;
		grenade.phobj.impactx = toleft ? -speed : speed;
		grenade.phobj.impacty = -speed;

		grenades.push_back(grenade);
	}

	void Combat::update_grenades()
	{
		for (auto iter = grenades.begin(); iter != grenades.end();)
		{
			Grenade &grenade = *iter;
			grenade.ball.update();
			physics.move_object(grenade.phobj);

			// It goes off as it lands on a platform or hits a wall
			// (0x9ba95a).
			if (grenade.phobj.onground || grenade.phobj.walled)
			{
				Grenade done = grenade;
				iter = grenades.erase(iter);
				explode(done.owner, done.skill, done.phobj.get_position());
			} else
			{
				++iter;
			}
		}
	}

	void Combat::explode(int32_t owner, int32_t skillid, Point<int16_t> position)
	{
		const Skill &skill = get_skill(skillid);
		fieldeffects.add(skill.get_special(), DrawArgument(position), 0);

		if (owner != player.get_oid())
			return;

		skill.play_use2();

		// It hits the mobs around it as a swing would, in front of where
		// the player now faces, all at once and with no charge.
		Attack attack = player.prepare_attack(skillid);
		skill.apply_stats(player, attack);
		attack.charge = 0;
		attack.origin = position;

		Rectangle<int16_t> range = place_range(position, attack.range, 1.0f, attack.toleft);
		std::vector<int32_t> targets = find_targets(mobs.get_mobs(), range, position, attack.mobcount, true, false);

		AttackResult result = attack;
		mobs.send_attack(result, attack, targets, attack.mobcount);
		result.attacker = player.get_oid();
		expect_hits(result, 1);
		extract_effects(player, skill, result, range, 0, range, position);

		AttackPacket(result, player.get_position(), position).dispatch();
	}

	void Combat::clear_grenades()
	{
		grenades.clear();
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
