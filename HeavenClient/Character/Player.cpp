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

#include "AutoPotions.h"
#include "Berserk.h"
#include "CatchItemRules.h"
#include "MorphRules.h"
#include "MobDamage.h"
#include "PlayerStates.h"
#include "SecondaryStat.h"
#include "SkillId.h"
#include "Inventory/ItemExpiry.h"
#include "Inventory/ItemProperties.h"

#include "../Audio/Audio.h"
#include "../Configuration.h"
#include "../Data/BulletData.h"
#include "../Data/EquipData.h"
#include "../Data/ItemData.h"
#include "../Data/PetData.h"
#include "../Data/SkillData.h"
#include "../Data/WeaponData.h"
#include "../Gameplay/Combat/Combo.h"
#include "../Gameplay/Combat/Dash.h"
#include "../Gameplay/Combat/EnergyCharge.h"
#include "../Gameplay/Combat/FlashJump.h"
#include "../Gameplay/Combat/RecoilShot.h"
#include "../Gameplay/Combat/Wings.h"
#include "../Gameplay/Combat/MagicDamage.h"
#include "../Gameplay/Combat/PqSkills.h"
#include "../Gameplay/MapleMap/FieldLimit.h"
#include "../Gameplay/Stage.h"
#include "../IO/Messages.h"
#include "../IO/UI.h"

#include "../IO/UITypes/UIChatBar.h"
#include "../IO/UITypes/UINotice.h"
#include "../IO/UITypes/UIRevive.h"
#include "../IO/UITypes/UISkillBook.h"
#include "../IO/UITypes/UIStatsInfo.h"
#include "../IO/UITypes/UIStatusBar.h"
#include "../IO/UITypes/UISystemOption.h"
#include "../Net/Packets/AttackAndSkillPackets.h"
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

		// A notice, as for a catch item or a pet's potion, in the chat line v83 uses for them (type 0xc).
		void catch_notice(const std::string &line)
		{
			if (line.empty())
				return;

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(line, UIChatBar::LineType::SKILL);
		}

		// Whether the item `create` a catch makes would fit: an equip needs an empty slot, anything else an
		// empty slot or a stack below its slotMax (FUN_00a09bdf, 0xa09cf2-0xa09e19).
		bool catch_fits(const Inventory &inventory, int32_t create)
		{
			InventoryType::Id type = InventoryType::by_item_id(create);

			if (inventory.find_free_slot(type))
				return true;

			if (create / 1000000 < 2 || create / 1000000 > 4)
				return false;

			static const char *const dirs[] = { "", "", "Consume", "Install", "Etc" };
			std::string strid = "0" + std::to_string(create);
			nl::node max = nl::nx::item[dirs[create / 1000000]][strid.substr(0, 4) + ".img"][strid]["info"]["slotMax"];
			int64_t most = max ? max.get_integer() : 100;

			for (int16_t slot = 1; slot <= inventory.get_slotmax(type); slot++)
				if (inventory.get_item_id(type, slot) == create && inventory.get_item_count(type, slot) < most)
					return true;

			return false;
		}

		// A mastery skill as the player has learned it.
		physical_damage::Learned learned(const SkillBook &skillbook, int32_t skill_id)
		{
			int32_t level = skillbook.get_level(skill_id);

			if (level <= 0)
				return {};

			const SkillData::Stats &stats = SkillData::get(skill_id).get_stats(level);

			return { stats.mastery, stats.x };
		}

		// A skill's damage or prop, in percent.
		int32_t in_percent(float value)
		{
			return static_cast<int32_t>(std::lround(value * 100));
		}

		// A USE item's node in Item.nx.
		nl::node consume_node(int32_t itemid)
		{
			return nl::nx::item["Consume"][string_format::extend_id(itemid / 10000, 4) + ".img"]
				[string_format::extend_id(itemid, 8)];
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
		phobj.wingsfall = 0.0;
		glide(false, false);
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

	void Player::recalc_stats()
	{
		Weapon::Type weapontype = get_weapontype();

		stats.set_weapontype(weapontype);
		stats.init_totalstats();

		// The ammo is picked again too, as a level up may allow more.
		inventory.recalc_stats(weapontype, stats.get_stat(MapleStat::Id::LEVEL));

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

		// Dash's speed and jump count only past a Speed or Jump buff's (0x8c457c, 0x8c45e8).
		stats.add_value(EquipStat::Id::SPEED, dash::speed_bonus(get_buff_value(Buffstat::Id::SPEED),
																 get_buff_value(Buffstat::Id::DASH_SPEED)));
		stats.add_value(EquipStat::Id::JUMP, dash::jump_bonus(get_buff_value(Buffstat::Id::JUMP),
															   get_buff_value(Buffstat::Id::DASH_JUMP)));

		// A full gauge lends Energy Charge's attack, accuracy and avoid past the buffs', and its prop as the
		// Stance chance when Power Stance is off (0x958bcc).
		if (energy_full())
		{
			int32_t skill = energy::skill(stats.get_job().get_id());
			const SkillData::Stats &own = SkillData::get(skill).get_stats(skillbook.get_level(skill));
			stats.add_value(EquipStat::Id::WATK, energy::bonus(get_buff_value(Buffstat::Id::WATK), own.pad));
			stats.add_value(EquipStat::Id::ACC, energy::bonus(get_buff_value(Buffstat::Id::ACC), own.acc));
			stats.add_value(EquipStat::Id::AVOID, energy::bonus(get_buff_value(Buffstat::Id::AVOID), own.eva));

			if (!has_buff(Buffstat::Id::STANCE))
				stats.set_stance(own.chance);
		}

		// The weapon's mastery adds its accuracy and an expert's attack for
		// the reach the job counts (SetFrom, FUN_0077f4c9), and the window
		// shows the range with the mastery of its own reach.
		int32_t job = stats.get_job().get_id();
		physical_damage::Weapon weapon = physical_damage::weapon(look.get_equips().get_weapon());
		auto lookup = [&](int32_t skill_id) { return learned(skillbook, skill_id); };
		physical_damage::Mastery counted = physical_damage::mastery(weapon, physical_damage::stat_reach(job), job, 0,
																	 lookup);
		stats.add_value(EquipStat::Id::ACC, counted.accuracy);
		stats.add_value(EquipStat::Id::WATK, counted.attack);
		stats.set_mastery(physical_damage::mastery(weapon, physical_damage::shown_reach(weapon), job, 0, lookup).mastery);

		// Shield Mastery raises the shield's own defence by its x percent,
		// and a pirate's bare hands have attack of their own.
		if (int32_t mastery = secondary_stat::shield_mastery(job))
		{
			int32_t percent = learned(skillbook, mastery).x;

			if (auto shield = inventory.get_equip(InventoryType::Id::EQUIPPED, EquipSlot::Id::SHIELD); shield && percent)
			{
				int32_t defence = shield->get_stat(EquipStat::Id::WDEF);
				stats.add_value(EquipStat::Id::WDEF, secondary_stat::shield_defence(defence, percent) - defence);
			}
		}

		if (look.get_equips().get_weapon() == 0)
			stats.add_value(EquipStat::Id::WATK,
							secondary_stat::bare_hands_attack(job, stats.get_stat(MapleStat::Id::LEVEL)));

		// Echo of Hero adds its percent of the whole attack and of the
		// whole magic, which counts INT (FUN_00791617).
		int32_t echo = get_buff_value(Buffstat::Id::ECHO_OF_HERO);
		int32_t magic = stats.get_total(EquipStat::Id::INT) + stats.get_total(EquipStat::Id::MAGIC);
		stats.add_value(EquipStat::Id::WATK, secondary_stat::echo_bonus(stats.get_total(EquipStat::Id::WATK), echo));
		stats.add_value(EquipStat::Id::MAGIC, secondary_stat::echo_bonus(magic, echo));

		stats.close_totalstats();

		if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
			statsinfo->update_all_stats();
	}

	void Player::change_equip(int16_t slot)
	{
		EquipSlot::Id part = EquipSlot::by_id(slot % Inventory::CASH_EQUIP);

		if (part == EquipSlot::Id::NONE)
			return;

		// The pets wear their own equipment.
		if (pets::is_equip_part(part))
		{
			look.set_pet_part(part, inventory.get_item_id(InventoryType::Id::EQUIPPED, inventory.shown_equip(part)));

			return;
		}

		// The look wears a cash item over the regular one, but has no cash
		// weapons (v83 draws them over the regular weapon's stances).
		int16_t shown = part == EquipSlot::Id::WEAPON ? part : inventory.shown_equip(part);

		if (part == EquipSlot::Id::WEAPON)
			look.set_cash_weapon(inventory.get_item_id(InventoryType::Id::EQUIPPED, part + Inventory::CASH_EQUIP));

		look.set_ring(part, inventory.get_item_id(InventoryType::Id::EQUIPPED, shown));

		if (int32_t itemid = inventory.get_item_id(InventoryType::Id::EQUIPPED, shown))
			look.add_equip(itemid);
		else
			look.remove_equip(part);
	}

	void Player::use_item(int32_t itemid)
	{
		InventoryType::Id type = InventoryType::by_item_id(itemid);

		if (int16_t slot = inventory.find_unsealed(type, itemid))
		{
			// v83 0x94f882: a reward box is sent before any other kind of item
			// is tried, as from a double-click.
			if (type == InventoryType::Id::USE && ItemData::get(itemid).has_reward())
				request(UseItemRewardPacket(slot, itemid), ExclRequest::USE_WAIT);
			else if (type == InventoryType::Id::USE && itemid / 10000 == 226)
				feed_mount(slot, itemid);
			else if (type == InventoryType::Id::USE && itemid / 10000 == 227)
				use_catch_item(slot, itemid);
			else if (type == InventoryType::Id::USE && !Stage::get().get_mapinfo().can_use_item(itemid))
				InChatMessage(Messages::ITEM_FIELD).drop();
			else if (type == InventoryType::Id::USE && field_limit::is_use_item(itemid) && ride_refuses(true))
				return;
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

			// A mount or the shoes grip the ground by their fs, spiked shoes
			// at 10, and swim at their swim percent of the speed, kicking at
			// the speed (FUN_0094d3d9).
			double grip = 1.0;
			int32_t swim = 100;

			if (ride_ability)
			{
				grip = ride_ability->fs;
				swim = ride_ability->swim;
			}
			else if (const MorphTemplate *morph = get_template_morph())
			{
				grip = morph->get_fs();
				swim = morph->get_swim();
			}
			else if (auto shoes = inventory.get_equip(InventoryType::Id::EQUIPPED, EquipSlot::Id::SHOES))
			{
				const EquipData &data = EquipData::get(shoes->get_item_id());
				grip = shoes->get_flags() & item_properties::EQUIP_SPIKES ? 10.0 : data.get_grip();
				swim = data.get_swim();
			}

			phobj.walkforce = grip;
			phobj.walkdrag = grip;
			phobj.swimspeed = phobj.walkspeed * swim / 100.0;
			phobj.swimkick = phobj.walkspeed;
			pst->update(*this);

			// v83 marks every knockback in the path as element 2 when it
			// lands on the player (0x9b222a).
			bool impact = phobj.impact;
			physics.move_object(phobj);

			if (impact && !phobj.impact)
				record_move(2);

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

		// A readied MP potion is checked each update, dead or alive (0x94b9b6).
		if (auto_mp_armed)
			check_auto_mp();

		// Landing or grabbing a rope readies Flash Jump again (0x9506f6).
		if (phobj.onground || is_climbing())
			flash_jumped = false;

		// They end Wings too, and so does its deadline once the player glides (0x9b4995, 0x94a834).
		wings_left -= Constants::TIMESTEP;

		if (phobj.onground || is_climbing() || wings::expired(wings_left, wings::gliding(phobj.wingsfall > 0.0, phobj.vspeed)))
			phobj.wingsfall = 0.0;

		glide(wings::gliding(phobj.wingsfall > 0.0, phobj.vspeed), phobj.onground);

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

	bool Player::may_flash_jump() const
	{
		bool swimming = phobj.type == PhysicsObject::Type::SWIMMING || phobj.type == PhysicsObject::Type::FLYING;

		return flash_jump::may_jump(phobj.onground, is_climbing(), flash_jumped, swimming, phobj.vspeed);
	}

	// Element 6 marks the jump in the path, before the push lands on the
	// next step (0x96bff3).
	void Player::flash_jump(int32_t level)
	{
		flash_jump::Impulse push = flash_jump::impulse(level, !facing_right);

		record_move(6);
		Physics::knockback(phobj, push.vx, push.vy);
		flash_jumped = true;
	}

	void Player::recoil_shot(int32_t level)
	{
		flash_jump::Impulse push = recoil_shot::backstep(level, !facing_right);

		record_move(recoil_shot::MOVE);
		Physics::knockback(phobj, push.vx, push.vy);
		flash_jumped = true;
	}

	bool Player::may_wings() const
	{
		return wings::may_cast(phobj.onground, phobj.swim, attacking);
	}

	// A plain jump, marked in the path by element 16, after which the path glides (FUN_009b21da).
	void Player::wings(int32_t x, int32_t seconds)
	{
		bool walking = is_key_down(KeyAction::Id::LEFT) || is_key_down(KeyAction::Id::RIGHT);
		Physics::jump(phobj, get_jumpforce(), walking ? (facing_right ? 1 : -1) : 0);

		Movement takeoff = current_move(wings::TAKEOFF);
		takeoff.vx = Movement::velocity(phobj.hspeed);
		takeoff.vy = Movement::velocity(phobj.vforce);
		path.add(takeoff);

		phobj.wingsfall = wings::fall_speed(x);
		wings_left = seconds * 1000;
	}

	Movement Player::current_move(uint8_t command) const
	{
		// While Wings is on, the path glides instead of walking and falling (0x9b10ee).
		if (command == 0 && phobj.wingsfall > 0.0)
			command = wings::GLIDE;

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

		// The buffs speed it up as far as 2 (FUN_00765066).
		return static_cast<int8_t>(std::clamp(base_speed + weapon_speed, 2, 10));
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

			// Only a swimmer or flier steers up and down; a dive held into a
			// landing must not pull the next jump down.
			if (st != Char::State::SWIM)
				phobj.flydir = 0;

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

		// Charged Blow needs a charge to spend (0x969545).
		if (move.get_id() == SkillId::CHARGE && !has_buff(Buffstat::Id::WK_CHARGE))
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (combo::enrage_refused(move.get_id(), get_buff_value(Buffstat::Id::COMBO)))
			return SpecialMove::ForbidReason::FBR_OTHER;

		if (energy::needs_full(move.get_id()) && !energy_full())
			return SpecialMove::ForbidReason::FBR_ENERGY;

		if (has_cooldown(move.get_id()))
			return SpecialMove::ForbidReason::FBR_COOLDOWN;

		int32_t level = skillbook.get_level(move.get_id());
		Weapon::Type weapon = get_weapontype();
		const Job &job = stats.get_job();
		uint16_t hp = stats.get_stat(MapleStat::Id::HP);
		// Infinity zeroes every skill's MP cost (FUN_00764256, 0x76446d).
		uint16_t mp = has_buff(Buffstat::Id::INFINITY_) ? UINT16_MAX : stats.get_stat(MapleStat::Id::MP);
		// Shadow Stars throws its star whatever a skill's cost, and Soul Arrow
		// needs no arrows at all (FUN_00949b5d, 0x969...). Once the star is
		// gone, v83 shows no message either: the shot just does nothing
		// (0x9698e2).
		bool stars = shadow_star() || shadow_star_lost();
		uint16_t bullets = stars || soul_arrow() ? UINT16_MAX : inventory.get_most_bullets();

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

	int32_t Player::partner_percent(int32_t skill) const
	{
		// A skill's x, a regular shot's y (0x7902d1).
		const SkillData::Stats &partner = skill_stats(skillbook, buffs[Buffstat::Id::SHADOWPARTNER].skillid);

		return skill ? partner.y : partner.x;
	}

	bool Player::soul_arrow() const
	{
		return has_buff(Buffstat::Id::SOULARROW);
	}

	int32_t Player::shadow_star() const
	{
		int16_t value = get_buff_value(Buffstat::Id::SHADOW_CLAW);

		if (value < 1 || get_weapontype() != Weapon::Type::CLAW)
			return 0;

		int32_t star = value + skill_use::SHADOW_STARS_BASE;

		return inventory.find_item(InventoryType::Id::USE, star) ? star : 0;
	}

	bool Player::shadow_star_lost() const
	{
		int16_t value = get_buff_value(Buffstat::Id::SHADOW_CLAW);
		bool held = value >= 1 && inventory.find_item(InventoryType::Id::USE, value + skill_use::SHADOW_STARS_BASE);

		return skill_use::star_lost(value, held);
	}

	Attack Player::prepare_attack(int32_t skill, bool whack) const
	{
		Attack::Type attacktype;
		int32_t star = shadow_star();
		int32_t bullet = star ? star : inventory.get_bulletid();

		if (state == Char::State::PRONE || whack)
		{
			attacktype = Attack::Type::CLOSE;
		} else if (skill)
		{
			// Like v83, a skill attacks as the skill does, whatever the weapon.
			const SkillData &data = SkillData::get(skill);

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
					// Soul Arrow shoots even with an empty quiver, and so
					// does Shadow Stars when its star is gone (FUN_0094e256).
					attacktype = inventory.has_projectile() || star || soul_arrow() || shadow_star_lost()
									 ? Attack::Type::RANGED
									 : Attack::Type::CLOSE;
					break;
				default:
					attacktype = Attack::Type::CLOSE;
					break;
			}
		}

		Attack attack;
		attack.type = attacktype;
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

		// What a blow comes from (FUN_0078df87): the attack without the
		// ammo the stats count, as a shot adds its own.
		using namespace physical_damage;

		Blow &blow = attack.blow;
		int32_t job = stats.get_job().get_id();
		Reach reach = attacktype == Attack::Type::RANGED ? SHOT : CLOSE;
		blow.weapon = physical_damage::weapon(look.get_equips().get_weapon());
		blow.job = job;
		blow.stats = { stats.get_total(EquipStat::Id::STR), stats.get_total(EquipStat::Id::DEX),
					   stats.get_total(EquipStat::Id::LUK) };
		blow.attack = stats.get_total(EquipStat::Id::WATK);

		if (int32_t counted = inventory.get_bulletid())
			blow.attack -= BulletData::get(counted).get_watk();

		blow.mastery = mastery(blow.weapon, reach, job, skill,
							   [&](int32_t skill_id) { return learned(skillbook, skill_id); }).mastery;

		int32_t critical = critical_skill(blow.weapon, reach, job);

		if (skillbook.get_level(critical) > 0)
		{
			const SkillData::Stats &source = skill_stats(skillbook, critical);
			blow.critical = { in_percent(source.chance), in_percent(source.damage), true };
		}

		blow.critical = sharpen(blow.critical, get_buff_value(Buffstat::Id::SHARP_EYES));

		// Stun Mastery crits on a stunned monster.
		if (skillbook.get_level(STUN_MASTERY) > 0)
		{
			const SkillData::Stats &source = skill_stats(skillbook, STUN_MASTERY);
			blow.stunned = { in_percent(source.chance), in_percent(source.damage), true };
		}

		if (has_buff(Buffstat::Id::WK_CHARGE))
		{
			int32_t charge = buffs[Buffstat::Id::WK_CHARGE].skillid;

			if (skillbook.get_level(charge) > 0)
			{
				const SkillData::Stats &source = skill_stats(skillbook, charge);
				blow.charge = { charge_element(charge), in_percent(source.damage), source.z };
			}
		}

		// Berserk, as the player last turned it on.
		blow.berserk = berserk;

		// Bullseye on the mob Homing Beacon marked, for any attack (FUN_0078dea6).
		attack.marked = guided.mob;

		if (skillbook.get_level(beacon::BULLSEYE) > 0)
			attack.bullseye = skill_stats(skillbook, beacon::BULLSEYE).x;

		blow.blind = has_buff(Buffstat::Id::DARKNESS);

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
		// A magic attack with an element is resisted by the job's resistance
		// skill (0x9583fa).
		int32_t resist = attack.magic ? resists(attack.element) : 0;

		int32_t damage = stats.calculate_damage(attack, randomizer.next_real(1.0), randomizer.next_real(1.0),
												 get_buff_value(Buffstat::Id::INVINCIBLE), resist);

		// Invincibility makes every mob's hit miss, even one Shadow Shifter
		// would dodge (FUN_00959708).
		bool divine = has_buff(Buffstat::Id::DIVINE_BODY);

		if (divine)
			damage = 0;

		if (mob_damage::blinded(attack.blind, randomizer.next_real(1.0)))
			damage = 0;

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

		if (shifter && !divine && skillbook.get_level(shifter) > 0 && randomizer.below(skill_stats(skillbook, shifter).chance))
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
		int32_t hp_loss = 0;
		bool guarded = false;

		if (dodged)
		{
			if (!blocked)
				set_expression(Expression::Id::SMILE, 1500);

			// Power Guard's reflect counts before a Guardian block takes it
			// back (0x959657, 0x959686).
			guarded = reflected.damage > 0;
		}
		else
		{
			int32_t achilles = 1000 - static_cast<int32_t>(std::lround(stats.get_reducedamage() * 1000.0f));
			int32_t mesoguard = get_buff_value(Buffstat::Id::MESOGUARD);
			int32_t shown = mob_damage::shown(damage, achilles, mesoguard, inventory.get_meso()) - reflected.damage;
			show_damage(shown);

			// Magic Guard's share shows, but comes out of MP (0x959595).
			int32_t magic_guard = mob_damage::magic_guarded(damage, get_buff_value(Buffstat::Id::MAGIC_GUARD),
					stats.get_stat(MapleStat::Id::MP), has_buff(Buffstat::Id::INFINITY_));
			hp_loss = std::max(shown - magic_guard, 0);
			guarded = reflected.damage > 0 || magic_guard > 0 ||
					  shown + reflected.damage < mob_damage::shown(damage, achilles, 0, 0);
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
			// An attack with info/knockback instead throws the player flat
			// and away from the mob, whichever way it faced. The path marks
			// the throw as element 12 (0x958b52).
			bool thrown = attack.knockback && attack.index >= 0;
			bool grounded = phobj.onground && !ladder;

			if (ladder)
			{
				set_state(Char::State::FALL);
				set_ladder(nullptr);
			}

			if (thrown)
			{
				record_move(12);
				Physics::knockback(phobj, mob_damage::thrown(grounded, attack.origin.x(), phobj.get_x()), 0.0);
			}
			else
			{
				bool guarded = attack.index < 0 && has_buff(Buffstat::Id::POWERGUARD);
				double power = guarded ? 200.0 : 270.0;

				Physics::knockback(phobj, fromleft ? -power : power, -power);
			}
		}

		uint8_t direction = fromleft ? 0 : 1;

		MobAttackResult result(attack, damage, direction, stance);

		// Without Power Stance, a full Energy Charge gauge lends its chance (0x958bc9).
		if (stance)
		{
			int32_t job = stats.get_stat(MapleStat::Id::JOB);
			result.stance_skill = has_buff(Buffstat::Id::STANCE) ? mob_damage::stance_skill(job) : energy::skill(job);
		}

		result.reflect = reflect;
		result.reflected = reflected.shown;
		result.guard = mob_damage::guard(blocked, attack.index < 0);
		result.dodged = dodged;
		result.hp_loss = hp_loss;
		result.guarded = guarded;
		result.position = get_position();

		return result;
	}

	std::pair<int32_t, bool> Player::damage_by_obstacle(int32_t damage, int32_t vx, int32_t vy)
	{
		// Achilles leaves the number as for a mob; Meso Guard has no part.
		int32_t achilles = 1000 - static_cast<int32_t>(std::lround(stats.get_reducedamage() * 1000.0f));
		int32_t shown = mob_damage::shown(damage, achilles, 0, 0);
		show_damage(shown);
		int32_t magic_guard = mob_damage::magic_guarded(damage, get_buff_value(Buffstat::Id::MAGIC_GUARD),
				stats.get_stat(MapleStat::Id::MP), has_buff(Buffstat::Id::INFINITY_));
		become_invincible(true);
		leave_chair();

		if ((vx != 0 || vy != 0) && state != Char::State::DIED)
		{
			if (ladder)
			{
				set_state(Char::State::FALL);
				set_ladder(nullptr);
			}

			Physics::knockback(phobj, vx, vy);
		}

		return { std::max(shown - magic_guard, 0), magic_guard > 0 };
	}

	void Player::give_buff(Buff buff)
	{
		buffs[buff.stat] = buff;

		if (buff.stat == Buffstat::Id::DARKSIGHT)
			dark_sight_at = now_ms();

		if (buff.stat == Buffstat::Id::COMBO)
			show_combo(buff.value - 1, false);

		if (buff.stat == Buffstat::Id::MORPH)
			transform(buff.value, false, underwater);

		if (buff.stat == Buffstat::Id::GHOST_MORPH)
			haunt(buff.value, false);

		if (buff.stat == Buffstat::Id::DIVINE_BODY)
			show_divine_body(pq_skills::family(stats.get_stat(MapleStat::Id::JOB), pq_skills::INVINCIBILITY));

		show_diseases();
	}

	int64_t Player::since_dark_sight() const
	{
		return dark_sight_at < 0 ? -1 : now_ms() - dark_sight_at;
	}

	void Player::cancel_buff(Buffstat::Id stat)
	{
		buffs[stat] = {};

		if (stat == Buffstat::Id::COMBO)
			show_combo(-1, false);

		if (stat == Buffstat::Id::MORPH)
			transform(0, false);

		if (stat == Buffstat::Id::GHOST_MORPH)
			haunt(0, false);

		if (stat == Buffstat::Id::DIVINE_BODY)
			show_divine_body(0);

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

		// A held walking key repeats, as Windows repeats it, but not while a
		// dialog is open: the field takes no key then (FUN_00529968).
		bool confused = has_buff(Buffstat::Id::CONFUSE);

		if (!UI::get().dialog_open() &&
			(is_held(KeyAction::Id::LEFT) || is_held(KeyAction::Id::RIGHT) ||
			 is_held(confused ? KeyAction::Id::UP : KeyAction::Id::DOWN)))
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

	int32_t Player::get_energy() const
	{
		return energy;
	}

	void Player::set_energy(int32_t value)
	{
		bool was_full = energy_full();
		energy = value;

		if (energy_full() != was_full)
			recalc_stats();
	}

	bool Player::energy_full() const
	{
		return energy >= energy::FULL && skillbook.get_level(energy::skill(stats.get_job().get_id())) > 0;
	}

	void Player::update_berserk(bool reset)
	{
		if (reset)
			berserk = 0;

		int32_t level = skillbook.get_level(SkillId::BERSERK);
		int32_t now = 0;

		if (level > 0)
		{
			const SkillData::Stats &source = skill_stats(skillbook, SkillId::BERSERK);
			now = berserk::damage(stats.get_job().get_id(), level, stats.get_stat(MapleStat::Id::HP),
								  stats.get_total(EquipStat::Id::HP), source.x, in_percent(source.damage));
		}

		if (!berserk::changed(berserk, now))
			return;

		berserk = now;
		CalcDamageStatPacket().dispatch();
	}

	int32_t Player::get_berserk() const
	{
		return berserk;
	}

	void Player::set_beacon(beacon::Mark mark)
	{
		guided = mark;
	}

	beacon::Mark Player::get_beacon() const
	{
		return guided;
	}

	void Player::change_skill(int32_t skill_id, int32_t skill_level, int32_t masterlevel, int64_t expiration)
	{
		int32_t old_level = skillbook.get_level(skill_id);
		skillbook.set_skill(skill_id, skill_level, masterlevel, expiration);

		if (old_level != skill_level)
			recalc_stats();
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

	int32_t Player::get_charge() const
	{
		return has_buff(Buffstat::Id::WK_CHARGE) ? buffs[Buffstat::Id::WK_CHARGE].skillid : 0;
	}

	int32_t Player::get_shadow_partner() const
	{
		return has_buff(Buffstat::Id::SHADOWPARTNER) ? shadow_partner::skill_for(stats.get_job().get_id()) : 0;
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

		if (!phobj.onground || is_sitting() || is_climbing() || attacking || is_dead() || ride_refuses() ||
			morph_refuses())
			return;

		if (!field_limit::may_sit(Stage::get().get_mapinfo().get_fieldtype()))
		{
			InChatMessage(Messages::Type::CHAIR_FIELD).drop();
			return;
		}

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
		if (ride_ability)
			return ride_ability->speed;

		return stats.get_total(EquipStat::Id::SPEED);
	}

	void Player::ask_ride(int32_t off)
	{
		if (off > 0)
			CancelBuffPacket(off).dispatch();

		ride_pending.asked(now_ms());
	}

	bool Player::is_ride_locked() const
	{
		return ride_pending.waiting(now_ms()) || is_switching_vehicle();
	}

	void Player::change_ride(int32_t v)
	{
		ride_pending.answered();

		// The ship come while climbing is sent back at once (decomp.c:1324318).
		if (mount::is_ship(v) && is_climbing())
		{
			ask_ride(mount::BATTLESHIP);
			return;
		}

		// The effect is the mount's got on, or the one got off.
		int32_t shown = v > 0 ? v : vehicle;

		if (shown <= 0)
			return;

		if (int32_t skill = mount::ride_effect_skill(shown, stats.get_stat(MapleStat::Id::JOB)))
			show_ride_effect(skill);

		ride(v, false);

		if (auto statsinfo = UI::get().get_element<UIStatsInfo>())
			statsinfo->update_all_stats();
	}

	bool Player::can_get_on() const
	{
		int32_t mob = inventory.get_item_id(InventoryType::Id::EQUIPPED, EquipSlot::Id::TAMEDMOB);
		int32_t saddle = inventory.get_item_id(InventoryType::Id::EQUIPPED, EquipSlot::Id::SADDLE);

		if (mob == 0 || saddle == 0)
		{
			InChatMessage(Messages::Type::RIDE_NO_SADDLE).drop();
			return false;
		}

		nl::node items = nl::nx::character["TamingMob"];
		auto info = [&items](int32_t id)
		{
			return items[string_format::extend_id(id, 8) + ".img"]["info"];
		};

		// The saddle, and anything else the mount wears (slot -20), have a least level for it.
		int32_t worn = inventory.get_item_id(InventoryType::Id::EQUIPPED, 20);
		bool ready = info(mob)["tamingMob"].get_integer() != 0 &&
			info(saddle)["reqMobLevel"].get_integer() <= mount_level &&
			(worn == 0 || info(worn)["reqMobLevel"].get_integer() <= mount_level) &&
			!(Stage::get().get_mapinfo().get_fieldlimit() & field_limit::RIDING);

		if (!ready)
			InChatMessage(Messages::Type::RIDE_REFUSED).drop();

		return ready;
	}

	bool Player::ride_refuses(bool monster_only) const
	{
		int32_t ridden = look.get_vehicle();
		bool monster = mount::is_tamed_monster(ridden);

		if (!monster && (monster_only || !mount::is_vehicle(ridden)))
			return false;

		InChatMessage(monster ? Messages::Type::RIDING_MONSTER : Messages::Type::RIDING_VEHICLE).drop();

		return true;
	}

	int32_t Player::resists(int32_t element) const
	{
		int32_t skill = mob_damage::resistance_skill(stats.get_stat(MapleStat::Id::JOB), element);

		return skill ? skill_stats(skillbook, skill).x : 0;
	}

	bool Player::morph_refuses() const
	{
		if (!get_template_morph())
			return false;

		InChatMessage(no_cancel_morph() ? Messages::Type::MORPHED_FIXED : Messages::Type::MORPHED).drop();

		return true;
	}

	namespace
	{
		// Whether a Use item may not be cancelled by its icon (noCancelMouse).
		bool no_cancel_mouse(int32_t item)
		{
			return item > 0 && nl::nx::item["Consume"][string_format::extend_id(item / 10000, 4) + ".img"]
				[string_format::extend_id(item, 8)]["info"]["noCancelMouse"].get_bool();
		}
	}

	bool Player::ghost_refuses() const
	{
		if (!is_ghost())
			return false;

		// Board the Spaceship (2360002) says the shorter line, as a fixed morph does (FUN_00a2a193).
		bool fixed = no_cancel_mouse(-buffs[Buffstat::Id::GHOST_MORPH].skillid);
		InChatMessage(fixed ? Messages::Type::MORPHED_FIXED : Messages::Type::MORPHED).drop();

		return true;
	}

	bool Player::shape_refuses() const
	{
		if (morph_refuses())
			return true;

		if (!is_superman())
			return false;

		InChatMessage(Messages::Type::MORPHED).drop();

		return true;
	}

	int32_t Player::get_morph_reason() const
	{
		return buffs[Buffstat::Id::MORPH].skillid;
	}

	bool Player::no_cancel_morph() const
	{
		return no_cancel_mouse(-get_morph_reason());
	}

	bool Player::ride_keeps(int32_t itemid)
	{
		int32_t type = itemid / 10000;

		if (!mount::is_tamed_monster(look.get_vehicle()) || (type != 190 && type != 191))
			return false;

		ask_ride(mount::rider_skill(stats.get_stat(MapleStat::Id::JOB)));

		return true;
	}

	void Player::feed_mount(int16_t slot, int32_t itemid)
	{
		if (mount::is_tamed_monster(look.get_vehicle()))
			request(MountFoodPacket(slot, itemid), ExclRequest::USE_WAIT);
	}

	void Player::use_catch_item(int16_t slot, int32_t itemid)
	{
		if (mount::is_tamed_monster(look.get_vehicle()) || itemid / 10000 != 227 || !can_request(ExclRequest::USE_WAIT))
			return;

		nl::node info = nl::nx::item["Consume"]["0227.img"]["0" + std::to_string(itemid)]["info"];

		if (!info)
			return;

		int32_t create = info["create"].get_integer();

		if (!catch_fits(inventory, create))
			return catch_notice(catch_item::no_room_notice(create));

		// The area is the player's, whichever way they face.
		Rectangle<int16_t> area(
			static_cast<int16_t>(info["left"].get_integer()),
			static_cast<int16_t>(info["right"].get_integer()),
			static_cast<int16_t>(info["top"].get_integer()),
			static_cast<int16_t>(info["bottom"].get_integer())
		);
		area.shift(get_position());

		int32_t oid = Stage::get().get_mobs().find_catchable(area, info["mob"].get_integer());

		if (!oid)
			return catch_notice(catch_item::no_mob_notice(info["bridleMsgType"].get_integer(), info["nomobMsg"].get_string()));

		request(UseCatchItemPacket(slot, itemid, oid), ExclRequest::USE_WAIT);
	}

	// CUserLocal clamps SPEED to 80-140 and JUMP to 80-123 before turning
	// them into shoe multipliers (FUN_007a4dec). A rider moves as the mount
	// does instead, and an item's morph or the barrel as its template
	// (FUN_0094d3d9).
	double Player::get_walkspeed() const
	{
		if (ride_ability)
			return ride_ability->speed / 100.0;

		if (const MorphTemplate *morph = get_template_morph())
			return morph::speed(morph->get_speed()) / 100.0;

		std::optional<int32_t> slow;

		if (has_buff(Buffstat::Id::SLOW))
			slow = get_buff_value(Buffstat::Id::SLOW);

		int32_t speed = disease::walk_speed(stats.get_total(EquipStat::Id::SPEED), slow);

		return std::clamp<int32_t>(speed, 80, 140) / 100.0;
	}

	double Player::get_jumpforce() const
	{
		if (ride_ability)
			return Physics::jump_speed(ride_ability->jump / 100.0);

		if (const MorphTemplate *morph = get_template_morph())
			return Physics::jump_speed(morph::jump(morph->get_jump()) / 100.0);

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

	void Player::set_underwater(bool uw)
	{
		underwater = uw;
		phobj.swim = uw;
	}

	void Player::set_flymap(bool flymap)
	{
		phobj.flymap = flymap;
	}

	bool Player::is_afloat() const
	{
		return underwater || phobj.flymap;
	}

	bool Player::may_jump_afloat()
	{
		if (!is_afloat())
			return true;

		int64_t now = now_ms();

		if (now - jumped_afloat_at < JUMP_AFLOAT_DELAY)
			return false;

		jumped_afloat_at = now;

		return true;
	}

	bool Player::is_key_down(KeyAction::Id action) const
	{
		switch (action)
		{
		case KeyAction::Id::LEFT:
		case KeyAction::Id::RIGHT:
		case KeyAction::Id::UP:
		case KeyAction::Id::DOWN:
			// v83 reads no arrow while a dialog is open, so the player
			// stops walking, crouching or climbing (FUN_009cbefb).
			if (UI::get().dialog_open())
				return false;

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

	LookEntry Player::look_entry() const
	{
		LookEntry entry;
		entry.female = stats.get_female();
		entry.skin = static_cast<uint8_t>(look.get_skin_id());
		entry.faceid = look.get_face_id();
		entry.hairid = look.get_hair_id();

		for (int16_t part = 1; part < Inventory::CASH_EQUIP; part++)
		{
			int32_t worn = inventory.get_item_id(InventoryType::Id::EQUIPPED, part);
			int32_t cash = inventory.get_item_id(InventoryType::Id::EQUIPPED, part + Inventory::CASH_EQUIP);
			auto slot = static_cast<int8_t>(part);

			if (part == EquipSlot::Id::WEAPON)
			{
				if (worn)
					entry.equips[slot] = worn;

				entry.maskedequips[-111] = cash;
			} else if (cash)
			{
				entry.equips[slot] = cash;

				if (worn)
					entry.maskedequips[slot] = worn;
			} else if (worn)
			{
				entry.equips[slot] = worn;
			}
		}

		return entry;
	}

	bool Player::pet_wears(uint8_t index, int32_t itemid) const
	{
		auto part = static_cast<EquipSlot::Id>((*pets::equip_parts(itemid))[index]);

		return inventory.get_item_id(InventoryType::Id::EQUIPPED, inventory.shown_equip(part)) == itemid;
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

	void Player::pets_say(const std::string &text, bool censored)
	{
		for (size_t i = 0; i < pets.size(); i++)
		{
			PetLook &pet = pets[i];

			if (!pet.get_itemid())
				break;

			// A pet wearing a quote ring talks in its kind's balloon.
			bool styled = look.get_pet_parts().count(pets::quote_part(i)) > 0;
			int16_t level = inventory.get_pet_level(pet.get_serial());

			if (std::optional<PetLook::Said> said = pet.say(text, censored, styled, level))
				PetChatPacket(pet.get_serial(), said->type, said->action, said->text).dispatch();
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

	int32_t Player::get_auto_potion(bool mp) const
	{
		return auto_potions[mp];
	}

	void Player::set_auto_potion(bool mp, int32_t itemid)
	{
		auto_potions[mp] = itemid;
	}

	void Player::change_auto_potion(bool mp, int32_t itemid)
	{
		bool restores = false;

		if (itemid != 0)
		{
			nl::node spec = consume_node(itemid)["spec"];
			restores = mp ? spec["mp"] || spec["mpR"] : spec["hp"] || spec["hpR"];
		}

		switch (auto_potion::change(itemid, wears_pouch(mp), restores, auto_potions[mp]))
		{
			case auto_potion::Change::CLEAR:
				auto_potions[mp] = 0;
				ChangeAutoPotPacket(mp, 0).dispatch();
				break;
			case auto_potion::Change::REFUSED:
				UI::get().emplace<UIOk>(mp ? auto_potion::ONLY_MP : auto_potion::ONLY_HP, [](bool) {});
				break;
			case auto_potion::Change::SET:
				// Then the alerts the potion waits on may be set.
				auto_potions[mp] = itemid;
				ChangeAutoPotPacket(mp, itemid).dispatch();
				catch_notice(mp ? auto_potion::SET_MP : auto_potion::SET_HP);
				UI::get().emplace<UISystemOption>();
				break;
			default:
				break;
		}
	}

	bool Player::wears_pouch(bool mp) const
	{
		return pets[0].get_itemid() && pet_wears(0, mp ? auto_potion::MP_POUCH : auto_potion::HP_POUCH);
	}

	void Player::check_auto_hp(int32_t hp_loss, bool guarded)
	{
		int32_t itemid = auto_potions[0];

		if (itemid && wears_pouch(false) && !get_diseases().seduced &&
			auto_potion::below(stats.get_stat(MapleStat::Id::HP), hp_loss, stats.get_total(EquipStat::Id::HP),
							   Setting<HPAlert>::get().load()))
		{
			// The misses start over even if the request can't go yet.
			if (int16_t slot = inventory.find_item(InventoryType::Id::USE, itemid))
			{
				give_potion(false, slot, itemid);
				potion_misses[0] = 0;
			}
			else
			{
				lack_potion(false);
			}
		}

		if (guarded)
		{
			arm_auto_mp();
			check_auto_mp();
		}
	}

	void Player::arm_auto_mp()
	{
		auto_mp_armed = true;
	}

	void Player::check_auto_mp()
	{
		int32_t itemid = auto_potions[1];

		if (!auto_mp_armed || !itemid || !wears_pouch(true) || get_diseases().seduced ||
			!auto_potion::below(stats.get_stat(MapleStat::Id::MP), 0, stats.get_total(EquipStat::Id::MP),
								Setting<MPAlert>::get().load()))
			return;

		if (int16_t slot = inventory.find_item(InventoryType::Id::USE, itemid))
			give_potion(true, slot, itemid);
		else
			lack_potion(true);
	}

	void Player::forget_potion_misses()
	{
		potion_misses = { 0, 0 };
		auto_mp_armed = false;
	}

	void Player::give_potion(bool mp, int16_t slot, int32_t itemid)
	{
		// Unlike using the potion, Stop Potion doesn't stop the pet.
		if (Stage::get().get_mapinfo().get_fieldlimit() & field_limit::CONSUMABLES)
			return;

		if (stats.get_stat(MapleStat::Id::LEVEL) < consume_node(itemid)["info"]["reqLevel"].get_integer())
		{
			catch_notice(auto_potion::LEVEL_TOO_LOW);
			return;
		}

		int64_t now = now_ms();

		// An MP potion may go to a player who is down.
		if (!excl.can_send_potion(now) || (!mp && stats.get_stat(MapleStat::Id::HP) <= 0))
			return;

		PetAutoPotPacket(pets[0].get_serial(), slot, itemid).dispatch();
		excl.sent_potion(now, mp);
		Sound(itemid).play();

		if (mp)
			auto_mp_armed = false;
	}

	void Player::lack_potion(bool mp)
	{
		if (auto_potion::pets_complain(++potion_misses[mp]))
		{
			for (PetLook &pet : pets)
			{
				if (!pet.get_itemid())
					continue;

				Optional<const Pet> item = inventory.get_pet(pet.get_serial());

				if (item && item->get_skill() & auto_potion::SPEAKS_UP)
					pet.act({ 3, 0, PetData::get(pet.get_itemid()).get_no_potion(mp) }, true);
			}
		}

		catch_notice(mp ? auto_potion::LACKING_MP : auto_potion::LACKING_HP);
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

	bool Player::quest_start_met(const QuestInfo &quest) const
	{
		auto done = questlog.completed_quests().find(quest.id);

		int64_t now = item_expiry::now();

		if (now < quest.start_time)
			return false;

		if (done != questlog.completed_quests().end() && !repeat_due(quest, done->second, now))
			return false;

		auto worn = [&](int32_t id) { return inventory.count_slots(InventoryType::Id::EQUIPPED, id) > 0; };

		return start_holdings_met(quest, stats.get_stat(MapleStat::Id::FAME),
			[&](int32_t id) { return inventory.get_total_item_count(id) + (worn(id) ? 1 : 0); },
			[&](int32_t id) { return skillbook.has_skill(id) || !SkillData::get(id).is_invisible(); }, worn,
			Stage::get().get_mapid(), first_pet());
	}

	FirstPet Player::first_pet() const
	{
		return { pets[0].get_itemid(), get_pet_closeness()[0] };
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

	void Player::set_linked_name(const std::string &name)
	{
		linked_name = name;
	}

	const std::string &Player::get_linked_name() const
	{
		return linked_name;
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

	Alliance &Player::get_alliance()
	{
		return alliance;
	}

	const Alliance &Player::get_alliance() const
	{
		return alliance;
	}

	Family &Player::get_family()
	{
		return family;
	}

	const Family &Player::get_family() const
	{
		return family;
	}

	const std::optional<MarriageRecord> &Player::get_marriage() const
	{
		return marriage;
	}

	void Player::set_marriage(std::optional<MarriageRecord> record)
	{
		marriage = std::move(record);
	}

	const std::vector<CoupleRingRecord> &Player::get_couple_rings() const
	{
		return couple_rings;
	}

	void Player::set_couple_rings(std::vector<CoupleRingRecord> records)
	{
		couple_rings = std::move(records);
	}

	std::optional<RingWearer> Player::ring_wearer(RingKind kind) const
	{
		static constexpr EquipSlot::Id RINGS[] = { EquipSlot::Id::RING1, EquipSlot::Id::RING2, EquipSlot::Id::RING3,
												   EquipSlot::Id::RING4 };

		if (kind == RingKind::WEDDING)
		{
			// A wedding ring is worn in a regular slot.
			bool wearing = false;

			for (auto slot : RINGS)
				wearing |= marriage::is_wedding_ring(inventory.get_item_id(InventoryType::Id::EQUIPPED, slot));

			if (auto ring = marriage::own_ring(marriage, stats.get_female(), wearing))
				return RingWearer::of(get_oid(), get_position(), *ring);

			return std::nullopt;
		}

		// Crush and friendship rings are cash items, worn 100 above the slot.
		std::vector<std::pair<int32_t, int64_t>> worn;

		for (auto slot : RINGS)
		{
			int16_t position = slot + Inventory::CASH_EQUIP;

			if (int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIPPED, position))
				worn.emplace_back(item_id, inventory.get_cash_id(InventoryType::Id::EQUIPPED, position));
		}

		if (auto ring = marriage::own_pair_ring(kind, worn, couple_rings))
			return RingWearer::of(get_oid(), get_position(), *ring);

		return std::nullopt;
	}

	const PartnerSpot &Player::get_partner() const
	{
		return partner;
	}

	void Player::set_partner(PartnerSpot spot)
	{
		partner = spot;
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