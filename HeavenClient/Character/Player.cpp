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

#include "PlayerStates.h"

#include "../Audio/Audio.h"
#include "../Data/EquipData.h"
#include "../Data/SkillData.h"
#include "../Data/WeaponData.h"
#include "../Gameplay/Stage.h"
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
		keysdown.clear();
		attacking = false;
		ladder = nullptr;
		set_chair(0);
		nullstate.update_state(*this);
	}

	void Player::send_action(KeyAction::Id action, bool down)
	{
		const PlayerState *pst = get_state(state);

		if (pst)
			pst->send_action(*this, action, down);

		keysdown[action] = down;
	}

	void Player::release_key(KeyAction::Id action)
	{
		if (is_key_down(action))
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
		if (int32_t itemid = inventory.get_item_id(InventoryType::Id::EQUIPPED, slot))
			look.add_equip(itemid);
		else
			look.remove_equip(EquipSlot::by_id(slot));
	}

	void Player::use_item(int32_t itemid)
	{
		InventoryType::Id type = InventoryType::by_item_id(itemid);

		if (int16_t slot = inventory.find_item(type, itemid))
		{
			if (type == InventoryType::Id::USE)
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

		uint8_t stancebyte = facing_right ? state : state + 1;
		Movement newmove(phobj, stancebyte);
		bool needupdate = lastmove.hasmoved(newmove);

		if (needupdate)
		{
			MovePlayerPacket(newmove).dispatch();
			lastmove = newmove;
		}

		return get_layer();
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

	bool Player::is_attacking() const
	{
		return attacking;
	}

	bool Player::can_attack() const
	{
		return !attacking && !is_dead() && !is_climbing() && !is_sitting();
	}

	SpecialMove::ForbidReason Player::can_use(const SpecialMove &move) const
	{
		// A spell cast while prone stabs instead (Combat::use_move).
		if (move.is_skill() && state == Char::State::PRONE && !SkillData::get(move.get_id()).is_spell())
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (move.is_attack() && (state == Char::State::LADDER || state == Char::State::ROPE))
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (has_cooldown(move.get_id()))
			return SpecialMove::ForbidReason::FBR_COOLDOWN;

		int32_t level = skillbook.get_level(move.get_id());
		Weapon::Type weapon = get_weapontype();
		const Job &job = stats.get_job();
		uint16_t hp = stats.get_stat(MapleStat::Id::HP);
		uint16_t mp = stats.get_stat(MapleStat::Id::MP);
		uint16_t bullets = inventory.get_most_bullets();

		return move.can_use(level, weapon, job, hp, mp, bullets);
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

	bool Player::is_dead() const
	{
		return state == Char::State::DIED;
	}

	bool Player::is_invincible() const
	{
		if (state == Char::State::DIED)
			return true;

		if (has_buff(Buffstat::Id::DARKSIGHT))
			return true;

		return Char::is_invincible();
	}

	MobAttackResult Player::damage(const MobAttack &attack)
	{
		int32_t damage = stats.calculate_damage(attack.watk);
		show_damage(damage);

		bool fromleft = attack.origin.x() > phobj.get_x();

		bool missed = damage <= 0;

		// Getting hurt knocks the player off a portable chair.
		if (!missed)
			leave_chair();

		bool knockback = !missed && state != Char::State::DIED;

		// CUserLocal knocks the player away from the mob and up at 270
		// pixels per second, or 200 under Power Guard, and off ladders and
		// ropes too (FUN_007ad352).
		if (knockback && randomizer.above(stats.get_stance()))
		{
			double power = has_buff(Buffstat::Id::POWERGUARD) ? 200.0 : 270.0;

			if (ladder)
			{
				set_state(Char::State::FALL);
				set_ladder(nullptr);
			}

			Physics::knockback(phobj, fromleft ? -power : power, -power);
		}

		uint8_t direction = fromleft ? 0 : 1;

		return {attack, damage, direction};
	}

	void Player::give_buff(Buff buff)
	{
		buffs[buff.stat] = buff;
	}

	void Player::cancel_buff(Buffstat::Id stat)
	{
		buffs[stat] = {};
	}

	bool Player::has_buff(Buffstat::Id stat) const
	{
		return buffs[stat].value > 0;
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
		show_effect_id(CharEffect::Id::JOBCHANGE);
		stats.change_job(jobid);
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
		return std::clamp<int32_t>(stats.get_total(EquipStat::Id::SPEED), 80, 140) / 100.0;
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

	BuddyList &Player::get_buddies()
	{
		return buddies;
	}

	const BuddyList &Player::get_buddies() const
	{
		return buddies;
	}

	Optional<const Ladder> Player::get_ladder() const
	{
		return ladder;
	}
}