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
#include "Player.h"

#include "MobDamage.h"
#include "PlayerStates.h"
#include "SkillId.h"

#include "../Audio/Audio.h"
#include "../Data/EquipData.h"
#include "../Data/SkillData.h"
#include "../Data/WeaponData.h"
#include "../Gameplay/Combat/Combo.h"
#include "../Gameplay/Combat/MagicDamage.h"
#include "../Gameplay/MapleMap/FieldLimit.h"
#include "../Gameplay/Stage.h"
#include "../IO/Messages.h"
#include "../IO/UI.h"

#include "../IO/UITypes/UIRevive.h"
#include "../IO/UITypes/UISkillBook.h"
#include "../IO/UITypes/UIStatsInfo.h"
#include "../IO/UITypes/UIStatusBar.h"
#include "../Net/Packets/GameplayPackets.h"
#include "../Net/Packets/PlayerPackets.h"
#include "../Net/Packets/InventoryPackets.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <chrono>
#include <cmath>

namespace ms
{
	const PlayerNullState nullstate;

	namespace
	{
		int64_t now_ms()
		{
			using namespace std::chrono;

			return duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count();
		}
	}

	const PlayerState *get_state(Char::State state)
	{
		static PlayerStandState standing;
		static PlayerWalkState walking;
		static PlayerFallState falling;
		static PlayerProneState lying;
		static PlayerClimbState climbing;
		static PlayerSitState sitting;
		static PlayerFlyState flying;

		switch (state)
		{
			case Char::State::STAND:
				return &standing;
			case Char::State::WALK:
				return &walking;
			case Char::State::FALL:
				return &falling;
			case Char::State::PRONE:
				return &lying;
			case Char::State::LADDER:
			case Char::State::ROPE:
				return &climbing;
			case Char::State::SIT:
				return &sitting;
			case Char::State::SWIM:
				return &flying;
			default:
				return nullptr;
		}
	}

	Player::Player(const CharEntry &entry) : Char(entry.id, entry.look, entry.stats.name), stats(entry.stats)
	{
		attacking = false;
		underwater = false;
		phobj.shoe = true;

		set_state(Char::State::STAND);
		set_direction(true);
	}

	Player::Player() : Char(0, {}, "")
	{}

	void Player::respawn(Point<int16_t> pos, bool uw)
	{
		set_position(pos.x(), pos.y());
		underwater = uw;
		phobj.swim = uw;
		phobj.flydir = 0;
		keysdown.clear();
		attacking = false;
		ladder = nullptr;
		set_chair(0);
		nullstate.update_state(*this);
		path.reset(current_move(0));
		stirred = now_ms();
	}

	void Player::send_action(KeyAction::Id action, bool down)
	{
		const PlayerState *pst = get_state(state);
		bool refused = action == KeyAction::Id::JUMP && !disease::can_jump(get_diseases());

		if (pst && !refused)
			pst->send_action(*this, action, down);

		// Under Stop Motion, a walking key going down, repeating or up
		// (FUN_0094c856, after Confuse swaps them), or a jump, is moving.
		bool confused = has_buff(Buffstat::Id::CONFUSE);
		bool walks = action == KeyAction::Id::LEFT || action == KeyAction::Id::RIGHT ||
					 action == (confused ? KeyAction::Id::UP : KeyAction::Id::DOWN);

		if (has_buff(Buffstat::Id::STOP_MOTION) && !is_dead() &&
			(walks || (action == KeyAction::Id::JUMP && down && !refused)))
			stirred = now_ms();

		keysdown[action] = down;
	}

	void Player::release_key(KeyAction::Id action)
	{
		if (is_held(action))
			send_action(action, false);
	}

	void Player::release_keys()
	{
		std::vector<KeyAction::Id> held;

		for (const auto &key : keysdown)
			if (key.second)
				held.push_back(key.first);

		for (KeyAction::Id action : held)
			send_action(action, false);
	}

	void Player::recalc_stats(bool equipchanged)
	{
		Weapon::Type weapontype = get_weapontype();

		stats.set_weapontype(weapontype);
		stats.init_totalstats();

		if (equipchanged)
			inventory.recalc_stats(weapontype);

		for (auto stat : EquipStat::values)
		{
			int32_t inventory_total = inventory.get_stat(stat);
			stats.add_value(stat, inventory_total);
		}

		auto passive_skills = skillbook.collect_passives();

		for (auto &passive : passive_skills)
		{
			int32_t skill_id = passive.first;
			int32_t skill_level = passive.second;

			passive_buffs.apply_buff(stats, skill_id, skill_level);
		}

		for (const Buff &buff : buffs.values())
			active_buffs.apply_buff(stats, buff.stat, buff.value);

		stats.close_totalstats();

		if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
			statsinfo->update_all_stats();
	}

	void Player::change_equip(int16_t slot)
	{
		EquipSlot::Id part = EquipSlot::by_id(slot % Inventory::CASH_EQUIP);

		if (part == EquipSlot::Id::NONE)
			return;

		// The pets wear their own equipment, and the look carries only
		// their clothes.
		if (pets::is_equip_part(part))
		{
			if (int8_t index = pets::wear_owner(part); index >= 0)
				look.set_pet_wear(index, inventory.get_item_id(InventoryType::Id::EQUIPPED, inventory.shown_equip(part)));

			return;
		}

		// The look wears a cash item over the regular one, but has no cash
		// weapons (v83 draws them over the regular weapon's stances).
		int16_t shown = part == EquipSlot::Id::WEAPON ? part : inventory.shown_equip(part);

		if (int32_t itemid = inventory.get_item_id(InventoryType::Id::EQUIPPED, shown))
			look.add_equip(itemid);
		else
			look.remove_equip(part);
	}

	void Player::use_item(int32_t itemid)
	{
		InventoryType::Id type = InventoryType::by_item_id(itemid);

		if (int16_t slot = inventory.find_item(type, itemid))
		{
			if (type == InventoryType::Id::USE && !Stage::get().get_mapinfo().can_use_item(itemid))
				InChatMessage(Messages::ITEM_FIELD).drop();
			else if (type == InventoryType::Id::USE && !disease::can_use_item(get_diseases()))
				return;
			else if (type == InventoryType::Id::USE && pets::is_food(itemid))
				feed_pets(slot, itemid);
			else if (type == InventoryType::Id::USE && field_limit::is_use_item(itemid))
				request(UseItemPacket(slot, itemid), ExclRequest::USE_WAIT);
			else if (type == InventoryType::Id::SETUP && itemid / 10000 == 301)
				sit_on_chair(itemid);
		}
	}

	bool Player::can_request(int32_t wait, bool allow_dead) const
	{
		return (allow_dead || stats.get_stat(MapleStat::Id::HP) > 0) && excl.can_send(now_ms(), wait);
	}

	bool Player::request(OutPacket &&packet, int32_t wait, bool allow_dead)
	{
		if (!can_request(wait, allow_dead))
			return false;

		packet.dispatch();
		excl.sent(now_ms());

		return true;
	}

	void Player::request_answered()
	{
		excl.answered(now_ms());
	}

	void Player::clear_request()
	{
		excl.clear();
	}

	void Player::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		if (layer == get_layer())
			Char::draw(viewx, viewy, alpha);
	}

	int8_t Player::update(const Physics &physics)
	{
		update_death();

		const PlayerState *pst = get_state(state);

		if (pst)
		{
			// A seduction that jumps keeps the player jumping.
			disease::State diseases = get_diseases();

			if (diseases.seduced && disease::seduced(*diseases.seduced).jump && phobj.onground)
				pst->send_action(*this, KeyAction::Id::JUMP, true);

			phobj.walkspeed = get_walkspeed();
			pst->update(*this);
			physics.move_object(phobj);

			bool aniend = Char::update(physics, get_stancespeed());

			if (aniend && attacking)
			{
				attacking = false;
				nullstate.update_state(*this);
			} else
			{
				pst->update_state(*this);
			}
		} else if (state == Char::State::DIED)
		{
			physics.move_object(phobj);
			Char::update(physics, 1.0f);
		}

		update_recovery();

		path.step(current_move(0), Constants::TIMESTEP);

		if (path.ready())
		{
			move_path::Path sent = path.flush();

			if (!sent.idle)
				MovePlayerPacket(sent).dispatch();
		}

		return get_layer();
	}

	uint8_t Player::get_stancebyte() const
	{
		return facing_right ? state : state + 1;
	}

	void Player::record_move(uint8_t command)
	{
		path.add(current_move(command));
	}

	Movement Player::current_move(uint8_t command) const
	{
		Movement move(phobj, get_stancebyte(), command, 0);

		if (is_dead())
			move.fh = 0;
		else if (ladder)
			move.fh = -ladder->get_id();
		else if (!phobj.onground)
			move.fh = 0;

		return move;
	}

	void Player::update_death()
	{
		bool dead = stats.get_stat(MapleStat::Id::HP) <= 0;

		if (dead != is_dead())
		{
			attacking = false;
			keysdown.clear();

			if (dead)
			{
				ladder = nullptr;
				phobj.hspeed = 0.0;
				phobj.type = PhysicsObject::Type::NORMAL;
				set_state(Char::State::DIED);
				Sound(Sound::Name::TOMBSTONE).play();
				death_ticks = 0;
			} else
			{
				nullstate.update_state(*this);
				UI::get().remove(UIElement::Type::REVIVE);
			}
		}

		// Like v83 (FUN_00a03350), offer the revive a little after the tombstone lands.
		if (dead && death_ticks <= REVIVE_DELAY && ++death_ticks > REVIVE_DELAY)
			UI::get().emplace<UIRevive>();
	}

	void Player::update_recovery()
	{
		int32_t hp = stats.get_stat(MapleStat::Id::HP);
		int32_t mp = stats.get_stat(MapleStat::Id::MP);
		double rate = recovery_rate;

		// The worn top's info/recovery (the sauna robes) only counts on a map
		// that already recovers faster (FUN_0083007f).
		if (rate > 1.0)
			if (int32_t top = inventory.get_item_id(InventoryType::Id::EQUIPPED, EquipSlot::Id::TOP))
				rate *= EquipData::get(top).get_recovery();

		if (chair)
			rate *= CHAIR_RECOVERY_RATE;

		// HP needs the player to keep still since the last frame. On a
		// ladder or rope only Endure heals; otherwise they must stand or sit,
		// not alert or attacking.
		bool still = phobj.get_x() == phobj.get_last_x() && phobj.get_y() == phobj.get_last_y();
		bool climbing = state == Char::State::LADDER || state == Char::State::ROPE;
		bool resting = state == Char::State::SIT || (state == Char::State::STAND && !look.get_alerted());
		uint16_t hp_interval = 0;

		if (hp > 0 && hp < stats.get_total(EquipStat::Id::HP) && still && !attacking)
		{
			if (climbing)
				hp_interval = get_endure_interval();
			else if (resting)
				hp_interval = stats.get_job().get_id() / 100 == 1 && skillbook.get_level(IMPROVED_HP_RECOVERY) > 0
							  ? WARRIOR_HP_RECOVERY_INTERVAL
							  : RECOVERY_INTERVAL;
		}

		int16_t heal_hp = 0;
		int16_t heal_mp = 0;

		if (hp_interval > 0)
		{
			int16_t natural = static_cast<int16_t>((RECOVERY_HP + get_recovery_hp_bonus()) * rate);
			heal_hp = hp_recovery.update(hp_interval, chair_recovery_hp, natural);
		} else
		{
			hp_recovery.timer = 0;
		}

		if (hp > 0 && mp < stats.get_total(EquipStat::Id::MP))
		{
			int16_t natural = static_cast<int16_t>((RECOVERY_MP + get_recovery_mp_bonus()) * rate);
			heal_mp = mp_recovery.update(RECOVERY_INTERVAL, chair_recovery_mp, natural);
		} else
		{
			mp_recovery.timer = 0;
		}

		// Like v83, HP and MP go out in separate packets, and an HP heal shows
		// its amount in blue.
		if (heal_hp > 0)
		{
			HealOverTimePacket(heal_hp, 0).dispatch();
			show_heal(heal_hp);
		}

		if (heal_mp > 0)
			HealOverTimePacket(0, heal_mp).dispatch();
	}

	int16_t Player::Recovery::update(uint16_t interval, int16_t chair_amount, int16_t natural_amount)
	{
		bool chair_source = chair_amount > 0;

		if (chair_source != from_chair)
		{
			from_chair = chair_source;
			timer = 0;
		}

		timer += Constants::TIMESTEP;

		if (timer < interval)
			return 0;

		timer = 0;

		return chair_source ? chair_amount : natural_amount;
	}

	namespace
	{
		// Whether job is base or advanced from it, as v83 checks (FUN_006545ca):
		// 410 covers 410 to 412, and 100 every warrior.
		bool in_job_line(int32_t job, int32_t base)
		{
			if (base % 100 == 0)
				return base / 100 == job / 100;

			return base / 10 == job / 10 && base % 10 <= job % 10;
		}

		const SkillData::Stats &skill_stats(const SkillBook &skillbook, int32_t skill_id)
		{
			return SkillData::get(skill_id).get_stats(skillbook.get_level(skill_id));
		}

		// The percent Element Amplification makes a spell, 100 without it
		// (FUN_00765771): the F/P, I/L and Blaze Wizard ones, each in its job.
		int32_t amplification(const SkillBook &skillbook, int32_t job)
		{
			constexpr std::pair<int32_t, int32_t> amplifications[] = {
					{211, 2110001},
					{221, 2210001},
					{1211, 12110001}
			};

			for (auto [line, skill] : amplifications)
				if (in_job_line(job, line) && skillbook.get_level(skill) > 0)
					return skill_stats(skillbook, skill).y;

			return 100;
		}
	}

	int16_t Player::get_recovery_hp_bonus() const
	{
		int32_t job = stats.get_job().get_id();

		if (job / 100 == 1)
			return static_cast<int16_t>(skill_stats(skillbook, IMPROVED_HP_RECOVERY).hp);

		if (in_job_line(job, 410))
			return static_cast<int16_t>(skill_stats(skillbook, ASSASSIN_ENDURE).hp);

		if (in_job_line(job, 420))
			return static_cast<int16_t>(skill_stats(skillbook, BANDIT_ENDURE).hp);

		return 0;
	}

	int16_t Player::get_recovery_mp_bonus() const
	{
		int32_t job = stats.get_job().get_id();

		// A magician recovers their skill level times their level over 10.
		if (job / 100 == 2)
			return static_cast<int16_t>(skillbook.get_level(IMPROVED_MP_RECOVERY) * get_level() / 10);

		if (in_job_line(job, 410))
			return static_cast<int16_t>(skill_stats(skillbook, ASSASSIN_ENDURE).mp);

		if (in_job_line(job, 420))
			return static_cast<int16_t>(skill_stats(skillbook, BANDIT_ENDURE).mp);

		if (in_job_line(job, 111))
			return static_cast<int16_t>(skill_stats(skillbook, FIGHTER_IMPROVED_MP_RECOVERY).mp);

		if (in_job_line(job, 121))
			return static_cast<int16_t>(skill_stats(skillbook, PAGE_IMPROVED_MP_RECOVERY).mp);

		return 0;
	}

	uint16_t Player::get_endure_interval() const
	{
		int32_t job = stats.get_job().get_id();
		int32_t skill_id = 0;

		if (job / 100 == 1)
			skill_id = WARRIOR_ENDURE;
		else if (in_job_line(job, 410))
			skill_id = ASSASSIN_ENDURE;
		else if (in_job_line(job, 420))
			skill_id = BANDIT_ENDURE;

		if (skill_id == 0 || skillbook.get_level(skill_id) <= 0)
			return 0;

		return static_cast<uint16_t>(skill_stats(skillbook, skill_id).time * 1000);
	}

	void Player::set_recovery_rate(double rate)
	{
		recovery_rate = rate;
	}

	int8_t Player::get_integer_attackspeed() const
	{
		int32_t weapon_id = look.get_equips().get_weapon();

		if (weapon_id <= 0)
			return 0;

		const WeaponData &weapon = WeaponData::get(weapon_id);

		int8_t base_speed = stats.get_attackspeed();
		int8_t weapon_speed = weapon.get_speed();

		return base_speed + weapon_speed;
	}

	void Player::set_direction(bool flipped)
	{
		if (!attacking)
			Char::set_direction(flipped);
	}

	void Player::set_state(State st)
	{
		if (!attacking)
		{
			Char::set_state(st);

			if (st != Char::State::SIT)
				set_chair(0);

			const PlayerState *pst = get_state(st);

			if (pst)
				pst->initialize(*this);
		}
	}

	void Player::end_attack()
	{
		Char::end_attack();
		nullstate.update_state(*this);
	}

	bool Player::is_attacking() const
	{
		return attacking;
	}

	bool Player::can_attack(bool interrupting) const
	{
		return (interrupting || !attacking) && !is_dead() && !is_climbing() && !is_sitting() && disease::can_attack(get_diseases());
	}

	SpecialMove::ForbidReason Player::can_use(const SpecialMove &move) const
	{
		// A spell cast while prone stabs instead (Combat::use_move). Teleport
		// works prone, so the player can hold Down and teleport down
		// (FUN_00957b74 never looks at the action).
		if (move.is_skill() && state == Char::State::PRONE && !SkillData::get(move.get_id()).is_spell()
			&& !teleport::teleports(move.get_id()))
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (move.is_attack() && (state == Char::State::LADDER || state == Char::State::ROPE))
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (move.is_skill() && !disease::can_use_skill(move.get_id(), get_diseases()))
			return SpecialMove::ForbidReason::FBR_OTHER;

		// A finisher needs an orb to spend (0x969623).
		if (combo::finisher(move.get_id()) && get_buff_value(Buffstat::Id::COMBO) <= 1)
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (has_cooldown(move.get_id()))
			return SpecialMove::ForbidReason::FBR_COOLDOWN;

		int32_t level = skillbook.get_level(move.get_id());
		Weapon::Type weapon = get_weapontype();
		const Job &job = stats.get_job();
		uint16_t hp = stats.get_stat(MapleStat::Id::HP);
		uint16_t mp = stats.get_stat(MapleStat::Id::MP);
		uint16_t bullets = inventory.get_most_bullets();

		SpecialMove::ForbidReason reason = move.can_use(level, weapon, job, hp, mp, bullets);

		// Dragon Roar costs its x percent of max HP instead, and needs at least half of it
		// (FUN_00764256).
		if (move.get_id() == SkillId::DRAGONS_ROAR && reason == SpecialMove::ForbidReason::FBR_NONE)
		{
			int32_t maxhp = stats.get_total(EquipStat::Id::HP);
			int32_t cost = SkillData::get(move.get_id()).get_stats(level).x * maxhp / 100;

			if (hp < maxhp / 2 || (cost != 0 && hp <= cost))
				reason = SpecialMove::ForbidReason::FBR_HPCOST;
		}

		// v83 checks the skill's mesos and item right after its HP and MP,
		// before the weapon (FUN_00764256).
		if (!move.is_skill() || reason == SpecialMove::ForbidReason::FBR_OTHER ||
			reason == SpecialMove::ForbidReason::FBR_HPCOST || reason == SpecialMove::ForbidReason::FBR_MPCOST)
			return reason;

		const SkillData::Stats &cost = SkillData::get(move.get_id()).get_stats(level);
		int32_t mesos = cost.mesocost;

		// Shadow Meso costs more while Shadow Partner is on.
		if (move.get_id() == SkillId::SHADOW_MESO && has_buff(Buffstat::Id::SHADOWPARTNER))
		{
			int32_t partner = skillbook.get_level(SkillId::SHADOW_PARTNER);
			mesos += SkillData::get(SkillId::SHADOW_PARTNER).get_stats(partner).y * mesos / 100;
		}

		if (mesos > 0 && inventory.get_meso() < mesos)
			return SpecialMove::ForbidReason::FBR_MESOCOST;

		if (cost.itemcost > 0 && cost.itemcount > 0 && inventory.get_total_item_count(cost.itemcost) < cost.itemcount)
			return SpecialMove::ForbidReason::FBR_ITEMCOST;

		return reason;
	}

	int32_t Player::combo_percent(int32_t skill) const
	{
		combo::Line line = combo::line(stats.get_job().get_id());
		int32_t level = skillbook.get_level(line.combo);

		if (level == 0)
			return 0;

		int32_t base_skill = combo::base_skill(line, level, skillbook.get_level(line.advanced));
		int32_t base_level = skillbook.get_level(base_skill);
		auto base = static_cast<int32_t>(std::lround(SkillData::get(base_skill).get_stats(base_level).damage * 100));
		int32_t orbs = get_buff_value(Buffstat::Id::COMBO) - 1;

		return combo::percent(combo::finisher(skill), level, base, orbs);
	}

	Attack Player::prepare_attack(int32_t skill, bool whack) const
	{
		Attack::Type attacktype;
		bool degenerate;
		int32_t bullet = inventory.get_bulletid();

		if (state == Char::State::PRONE || whack)
		{
			degenerate = true;
			attacktype = Attack::Type::CLOSE;
		} else if (skill)
		{
			// Like v83, a skill attacks as the skill does, whatever the weapon.
			const SkillData &data = SkillData::get(skill);
			degenerate = false;

			if (data.is_shot())
				attacktype = Attack::Type::RANGED;
			else if (data.is_spell())
				attacktype = Attack::Type::MAGIC;
			else
				attacktype = Attack::Type::CLOSE;

			if (!data.uses_ammo())
				bullet = 0;
		} else
		{
			Weapon::Type weapontype;
			weapontype = get_weapontype();

			switch (weapontype)
			{
				case Weapon::Type::BOW:
				case Weapon::Type::CROSSBOW:
				case Weapon::Type::CLAW:
				case Weapon::Type::GUN:
					degenerate = !inventory.has_projectile();
					attacktype = degenerate ? Attack::Type::CLOSE : Attack::Type::RANGED;
					break;
				case Weapon::Type::WAND:
				case Weapon::Type::STAFF:
					degenerate = true;
					attacktype = Attack::Type::CLOSE;
					break;
				default:
					attacktype = Attack::Type::CLOSE;
					degenerate = false;
					break;
			}
		}

		Attack attack;
		attack.type = attacktype;
		attack.mindamage = stats.get_mindamage();
		attack.maxdamage = stats.get_maxdamage();

		if (degenerate)
		{
			attack.mindamage /= 10;
			attack.maxdamage /= 10;
		}

		attack.critical = stats.get_critical();
		attack.ignoredef = stats.get_ignoredef();
		attack.accuracy = stats.get_total(EquipStat::Id::ACC);
		attack.playerlevel = stats.get_stat(MapleStat::Id::LEVEL);
		attack.intelligence = stats.get_total(EquipStat::Id::INT);
		attack.luck = stats.get_total(EquipStat::Id::LUK);
		attack.magic = magic_damage::magic(attack.intelligence, stats.get_total(EquipStat::Id::MAGIC));
		attack.amplify = amplification(skillbook, stats.get_job().get_id());
		attack.range = stats.get_range();
		attack.origin = get_position();
		attack.toleft = !facing_right;
		attack.speed = get_integer_attackspeed();

		// A swing or stab throws or fires nothing, even with ammo.
		if (attacktype == Attack::Type::RANGED)
			attack.bullet = bullet;

		return attack;
	}

	void Player::rush(double targetx)
	{
		if (phobj.onground)
		{
			uint16_t delay = get_attackdelay(1);
			phobj.movexuntil(targetx, delay);
			phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);
		}
	}

	MobAttackResult Player::damage(const MobAttack &attack)
	{
		int32_t damage = stats.calculate_damage(attack, randomizer.next_real(1.0), randomizer.next_real(1.0),
												 get_buff_value(Buffstat::Id::INVINCIBLE));

		// A deadly attack that hits leaves 1 HP, and an MP burn takes 1
		// (FUN_009581a9).
		if (damage > 0 && attack.deadly)
			damage = std::max(stats.get_stat(MapleStat::Id::HP) - 1, 1);
		else if (damage > 0 && attack.mpburn)
			damage = 1;

		// A Night Lord's or Shadower's Shadow Shifter may dodge any hit, a
		// miss too (0x9584f6). A Hero's or Paladin's Guardian may block one
		// that lands while they hold a shield, after Power Guard has sent its
		// share back (0x958692). Neither does, nor Power Stance, where the
		// map stops movement skills (0x9585a5, 0x958778, 0x958af6).
		bool procs = Stage::get().get_mapinfo().can_use_movement_skills();
		int32_t job = stats.get_stat(MapleStat::Id::JOB);
		int32_t dodged = 0;
		int32_t shifter = procs ? mob_damage::shadow_shifter(job) : 0;

		if (shifter && skillbook.get_level(shifter) > 0 && randomizer.below(skill_stats(skillbook, shifter).chance))
		{
			damage = -1;
			dodged = shifter;
		}

		int32_t guardian = procs ? mob_damage::guardian(job) : 0;
		bool blocked = damage > 0 && guardian && skillbook.get_level(guardian) > 0 &&
					   inventory.has_equipped(EquipSlot::Id::SHIELD) &&
					   randomizer.below(skill_stats(skillbook, guardian).chance / 10);

		// A touch under Power Guard sends some back at the mob, which the
		// player is spared (0x9587c6). A spell under Mana Reflection does
		// too if its chance comes up, sparing them nothing (0x958d50).
		uint8_t reflect = 0;
		mob_damage::Reflected reflected{ 0, 0 };
		mob_damage::Reflectee reflectee{ attack.level, attack.avoid, attack.maxhp, attack.fixeddamage, attack.boss,
									   attack.invincible };
		int32_t level = stats.get_stat(MapleStat::Id::LEVEL);

		if (attack.index < 0 && damage > 0 && has_buff(Buffstat::Id::POWERGUARD))
		{
			reflect = static_cast<uint8_t>(get_buff_value(Buffstat::Id::POWERGUARD));
			reflected = mob_damage::power_guard(damage, reflect, reflectee, level, stats.get_total(EquipStat::Id::ACC));
		}

		if (blocked)
		{
			damage = -1;
			dodged = guardian;
		}

		if (attack.index >= 0 && attack.magic && damage > 0 && has_buff(Buffstat::Id::MANA_REFLECTION))
		{
			const SkillData::Stats &mana =
					SkillData::get(SkillId::MANA_REFLECTION).get_stats(get_buff_value(Buffstat::Id::MANA_REFLECTION));

			if (randomizer.below(mana.chance))
			{
				reflect = static_cast<uint8_t>(mana.x);
				reflected.shown = mob_damage::mana_reflection(damage, mana.x, reflectee, level,
															  stats.get_total(EquipStat::Id::INT),
															  stats.get_total(EquipStat::Id::LUK));
			}
		}

		// A dodge shows no number; Shadow Shifter smiles (0x9593b9).
		if (dodged)
		{
			if (!blocked)
				set_expression(Expression::Id::SMILE, 1500);
		}
		else
		{
			int32_t achilles = 1000 - static_cast<int32_t>(std::lround(stats.get_reducedamage() * 1000.0f));
			show_damage(mob_damage::shown(damage, achilles, get_buff_value(Buffstat::Id::MESOGUARD),
										  inventory.get_meso()) - reflected.damage);
		}

		become_invincible(damage > 0);

		bool fromleft = attack.pushleft ? *attack.pushleft : attack.origin.x() > phobj.get_x();

		bool missed = damage <= 0;

		// Getting hurt knocks the player off a portable chair.
		if (!missed)
			leave_chair();

		bool knockback = !missed && state != Char::State::DIED;

		// Power Stance may keep the player in place; others see it too.
		bool stance = knockback && procs && !randomizer.above(stats.get_stance());

		// CUserLocal knocks the player away from the mob and up at 270
		// pixels per second, or 200 from a touch under Power Guard, and off
		// ladders and ropes too (FUN_007ad352, 0x9588cd).
		if (knockback && !stance)
		{
			bool guarded = attack.index < 0 && has_buff(Buffstat::Id::POWERGUARD);
			double power = guarded ? 200.0 : 270.0;

			if (ladder)
			{
				set_state(Char::State::FALL);
				set_ladder(nullptr);
			}

			Physics::knockback(phobj, fromleft ? -power : power, -power);
		}

		uint8_t direction = fromleft ? 0 : 1;

		MobAttackResult result(attack, damage, direction, stance);
		result.reflect = reflect;
		result.reflected = reflected.shown;
		result.guard = mob_damage::guard(blocked, attack.index < 0);
		result.dodged = dodged;
		result.position = get_position();

		return result;
	}

	void Player::give_buff(Buff buff)
	{
		buffs[buff.stat] = buff;
		show_diseases();
	}

	void Player::cancel_buff(Buffstat::Id stat)
	{
		buffs[stat] = {};
		show_diseases();
	}

	// A disease's reason is the mob skill that gave it.
	void Player::show_diseases()
	{
		std::vector<int32_t> reasons;

		for (Buffstat::Id stat : disease::STATS)
			if (has_buff(stat))
				reasons.push_back(buffs[stat].skillid);

		show_afflictions(reasons);
	}

	disease::State Player::get_diseases() const
	{
		disease::State diseases;
		diseases.stunned = has_buff(Buffstat::Id::STUN);
		diseases.sealed = has_buff(Buffstat::Id::SEAL);
		diseases.weakened = has_buff(Buffstat::Id::WEAKEN);
		diseases.confused = has_buff(Buffstat::Id::CONFUSE);

		diseases.stop_potion = has_buff(Buffstat::Id::STOP_POTION);

		if (has_buff(Buffstat::Id::SEDUCE))
			diseases.seduced = get_buff_value(Buffstat::Id::SEDUCE);

		if (has_buff(Buffstat::Id::STOP_MOTION))
			diseases.stop_motion = get_buff_value(Buffstat::Id::STOP_MOTION);

		return diseases;
	}

	int32_t Player::take_stop_motion()
	{
		std::optional<int32_t> n = get_diseases().stop_motion;

		if (!n || is_dead())
			return 0;

		int64_t now = now_ms();

		// A held walking key repeats, as Windows repeats it.
		bool confused = has_buff(Buffstat::Id::CONFUSE);

		if (is_held(KeyAction::Id::LEFT) || is_held(KeyAction::Id::RIGHT) ||
			is_held(confused ? KeyAction::Id::UP : KeyAction::Id::DOWN))
			stirred = now;

		if (now - stirred <= disease::STILL_TIME)
			return 0;

		int32_t damage = disease::stop_motion_damage(*n, stats.get_stat(MapleStat::Id::HP));

		if (damage > 0)
			stirred = now;

		return damage;
	}

	bool Player::has_buff(Buffstat::Id stat) const
	{
		return buffs[stat].value > 0;
	}

	int16_t Player::get_buff_value(Buffstat::Id stat) const
	{
		return buffs[stat].value;
	}

	void Player::change_skill(int32_t skill_id, int32_t skill_level, int32_t masterlevel, int64_t expiration)
	{
		int32_t old_level = skillbook.get_level(skill_id);
		skillbook.set_skill(skill_id, skill_level, masterlevel, expiration);

		if (old_level != skill_level)
			recalc_stats(false);
	}

	void Player::add_cooldown(int32_t skill_id, int32_t cooltime)
	{
		cooldowns[skill_id] = cooltime;
	}

	bool Player::has_cooldown(int32_t skill_id) const
	{
		auto iter = cooldowns.find(skill_id);

		if (iter == cooldowns.end())
			return false;

		return iter->second > 0;
	}

	void Player::change_level(uint16_t level)
	{
		uint16_t oldlevel = get_level();

		if (level > oldlevel)
		{
			show_effect_id(CharEffect::Id::LEVELUP);
			Sound(Sound::Name::LEVELUP).play();

			// Up to level 10, v83 also makes the stat key glow (FUN_00a1fb52).
			if (level < 11)
				if (auto statusbar = UI::get().get_element<UIStatusBar>())
					statusbar->show_key_hint(UIStatusBar::KEY_STAT);
		}

		stats.set_stat(MapleStat::Id::LEVEL, level);

		// Reaching level 2 opens a fresh skill window for the first SP, without a sound (FUN_00a1fb52).
		if (level == 2 && level > oldlevel)
		{
			auto window = UI::get().get_element<UISkillBook>();

			if (!window || !window->is_active())
			{
				UI::get().remove(UISkillBook::TYPE);
				UI::get().emplace<UISkillBook>(stats, skillbook);
			}
		}
	}

	uint16_t Player::get_level() const
	{
		return stats.get_stat(MapleStat::Id::LEVEL);
	}

	int32_t Player::get_skilllevel(int32_t skillid) const
	{
		return skillbook.get_level(skillid);
	}

	void Player::change_job(uint16_t jobid)
	{
		uint16_t oldjob = stats.get_stat(MapleStat::Id::JOB);

		show_effect_id(CharEffect::Id::JOBCHANGE);
		Sound(Sound::Name::JOBCHANGED).play();
		stats.change_job(jobid);

		// A beginner's first advancement opens the stat window, without a sound (FUN_00a1fb52).
		if (oldjob == 0 || oldjob == 1000 || oldjob == 2000 || oldjob == 2001)
		{
			auto window = UI::get().get_element<UIStatsInfo>();

			if (!window || !window->is_active())
			{
				UI::get().remove(UIStatsInfo::TYPE);
				UI::get().emplace<UIStatsInfo>(stats);
			}
		}
	}

	void Player::set_seat(Optional<const Seat> seat)
	{
		if (seat)
		{
			set_position(seat->getpos());
			set_state(Char::State::SIT);
		}
	}

	void Player::sit_on_chair(int32_t itemid)
	{
		// v83 sits only when it may ask the server, as for an item
		// (FUN_00a0f9e2).
		if (!can_request(ExclRequest::USE_WAIT))
			return;

		// Nor for the 5 seconds a hit or an attack keeps it alert.
		if (look.get_alerted())
			return;

		if (!phobj.onground || is_sitting() || is_climbing() || attacking || is_dead())
			return;

		nl::node info = nl::nx::item["Install"]["0301.img"][string_format::extend_id(itemid, 8)]["info"];

		if (stats.get_stat(MapleStat::Id::LEVEL) < info["reqLevel"].get_integer())
			return;

		phobj.hspeed = 0.0;
		set_state(Char::State::SIT);
		set_chair(itemid);
		request(UseChairPacket(itemid), ExclRequest::USE_WAIT);
	}

	void Player::leave_chair()
	{
		if (!chair || leaving_chair)
			return;

		leaving_chair = true;
		CancelChairPacket(-1).dispatch();
	}

	void Player::stand_up()
	{
		if (!is_sitting())
			return;

		set_state(Char::State::STAND);

		if (is_key_down(KeyAction::Id::LEFT) != is_key_down(KeyAction::Id::RIGHT))
		{
			set_direction(is_key_down(KeyAction::Id::RIGHT));
			set_state(Char::State::WALK);
		}
	}

	void Player::set_chair(int32_t itemid)
	{
		Char::set_chair(itemid);
		leaving_chair = false;

		nl::node info = nl::nx::item["Install"]["0301.img"][string_format::extend_id(itemid, 8)]["info"];
		chair_recovery_hp = itemid ? static_cast<int16_t>(info["recoveryHP"].get_integer()) : 0;
		chair_recovery_mp = itemid ? static_cast<int16_t>(info["recoveryMP"].get_integer()) : 0;
	}

	void Player::set_ladder(Optional<const Ladder> ldr)
	{
		ladder = ldr;

		if (ladder)
		{
			phobj.set_x(ldr->get_x());
			phobj.hspeed = 0.0;
			phobj.vspeed = 0.0;
			phobj.fhlayer = 7;
			phobj.fallstart = 0;
			set_state(ldr->is_ladder() ? Char::State::LADDER : Char::State::ROPE);
			set_direction(false);
		}
	}

	int32_t Player::get_walk_animation_speed() const
	{
		return stats.get_total(EquipStat::Id::SPEED);
	}

	// CUserLocal clamps SPEED to 80-140 and JUMP to 80-123 before turning
	// them into shoe multipliers (FUN_007a4dec).
	double Player::get_walkspeed() const
	{
		std::optional<int32_t> slow;

		if (has_buff(Buffstat::Id::SLOW))
			slow = get_buff_value(Buffstat::Id::SLOW);

		int32_t speed = disease::walk_speed(stats.get_total(EquipStat::Id::SPEED), slow);

		return std::clamp<int32_t>(speed, 80, 140) / 100.0;
	}

	double Player::get_jumpforce() const
	{
		return Physics::jump_speed(std::clamp<int32_t>(stats.get_total(EquipStat::Id::JUMP), 80, 123) / 100.0);
	}

	float Player::get_flyforce() const
	{
		return 0.25f;
	}

	bool Player::is_underwater() const
	{
		return underwater;
	}

	bool Player::is_key_down(KeyAction::Id action) const
	{
		switch (action)
		{
		case KeyAction::Id::LEFT:
		case KeyAction::Id::RIGHT:
		case KeyAction::Id::UP:
		case KeyAction::Id::DOWN:
			break;
		default:
			return is_held(action);
		}

		int8_t x = is_held(KeyAction::Id::RIGHT) - is_held(KeyAction::Id::LEFT);
		int8_t y = is_held(KeyAction::Id::DOWN) - is_held(KeyAction::Id::UP);
		disease::Input input = disease::steer(x, y, get_diseases());

		switch (action)
		{
		case KeyAction::Id::LEFT:
			return input.x < 0;
		case KeyAction::Id::RIGHT:
			return input.x > 0;
		case KeyAction::Id::UP:
			return input.y < 0;
		default:
			return input.y > 0;
		}
	}

	bool Player::is_held(KeyAction::Id action) const
	{
		return keysdown.count(action) ? keysdown.at(action) : false;
	}

	CharStats &Player::get_stats()
	{
		return stats;
	}

	const CharStats &Player::get_stats() const
	{
		return stats;
	}

	Inventory &Player::get_inventory()
	{
		return inventory;
	}

	const Inventory &Player::get_inventory() const
	{
		return inventory;
	}

	void Player::command_pets(const std::string &line)
	{
		for (PetLook &pet : pets)
		{
			if (!pet.get_itemid())
				continue;

			if (std::optional<pets::Heard> heard = pet.hear(line, inventory.get_pet_level(pet.get_serial())))
				PetCommandPacket(pet.get_serial(), heard->named, heard->command).dispatch();
		}
	}

	void Player::feed_pets(int16_t slot, int32_t itemid)
	{
		// Whether a pet is out is the server's to check.
		if (!request(PetFoodPacket(slot, itemid), ExclRequest::USE_WAIT))
			return;

		std::array<int16_t, 3> fullness;

		for (size_t i = 0; i < pets.size(); i++)
		{
			Optional<const Pet> pet = pets[i].get_itemid() ? inventory.get_pet(pets[i].get_serial()) : nullptr;
			fullness[i] = pet ? pet->get_fullness() : -1;
		}

		// The one the server will feed plays its eat sound, if it has one.
		int index = pets::hungriest(fullness);

		if (index >= 0)
			pets[index].play_eat_sound();
	}

	std::array<int32_t, 3> Player::get_pet_closeness() const
	{
		std::array<int32_t, 3> closeness;

		for (size_t i = 0; i < pets.size(); i++)
		{
			Optional<const Pet> pet = pets[i].get_itemid() ? inventory.get_pet(pets[i].get_serial()) : nullptr;
			closeness[i] = pet ? pet->get_closeness() : -1;
		}

		return closeness;
	}

	SkillBook &Player::get_skills()
	{
		return skillbook;
	}

	QuestLog &Player::get_quests()
	{
		return questlog;
	}

	TeleportRock &Player::get_teleport_rock()
	{
		return teleportrock;
	}

	MonsterBook &Player::get_monsterbook()
	{
		return monsterbook;
	}

	Party &Player::get_party()
	{
		return party;
	}

	const Party &Player::get_party() const
	{
		return party;
	}

	MysticDoor &Player::get_door()
	{
		return door;
	}

	const MysticDoor &Player::get_door() const
	{
		return door;
	}

	BuddyList &Player::get_buddies()
	{
		return buddies;
	}

	const BuddyList &Player::get_buddies() const
	{
		return buddies;
	}

	Guild &Player::get_guild()
	{
		return guild;
	}

	const Guild &Player::get_guild() const
	{
		return guild;
	}

	void Player::show_guild()
	{
		const GuildData &data = guild.get_data();

		set_guild_name(guild.exists() ? data.name : "");
		set_guild_emblem(data.emblem);
	}

	Optional<const Ladder> Player::get_ladder() const
	{
		return ladder;
	}
}