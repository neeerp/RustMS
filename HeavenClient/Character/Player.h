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
#pragma once

#include <cstdint>

#include "ActiveBuffs.h"
#include "BuddyList.h"
#include "ExclRequest.h"
#include "MonsterBook.h"
#include "Party.h"
#include "PassiveBuffs.h"
#include "QuestLog.h"
#include "SkillBook.h"
#include "TeleportRock.h"

#include "Inventory/Inventory.h"

#include "../Gameplay/MovePath.h"
#include "../Gameplay/Playable.h"

#include "../Gameplay/Combat/Skill.h"
#include "../Gameplay/MapleMap/Layer.h"
#include "../Gameplay/MapleMap/MapInfo.h"
#include "../Gameplay/Physics/Physics.h"
#include "../Net/OutPacket.h"

namespace ms
{
	class Player : public Playable, public Char
	{
	public:
		// Construct a player object from the given character entry.
		Player(const CharEntry &entry);

		Player();

		// Draw the player
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// Update the player's animation, physics and states.
		int8_t update(const Physics &physics) override;

		// Set flipped ignore if attacking.
		void set_direction(bool flipped) override;

		// Set state ignore if attacking.
		void set_state(State stance) override;

		// Respawn the player at the given position
		void respawn(Point<int16_t> position, bool underwater);

		// Mark where the player is now in the path to the server, as
		// element `command` of no duration (v83 FUN_0052ef17).
		void record_move(uint8_t command);

		// Sends a Keyaction to the player's state, to apply forces, change the state and other behaviour.
		void send_action(KeyAction::Id action, bool pressed);

		// Lets go of a held key, as when a dialog took its release.
		void release_key(KeyAction::Id action);

		// Lets go of every held key, as when the window loses focus.
		void release_keys();

		// Recalculates the total stats from base stats, inventories and skills.
		void recalc_stats(bool equipchanged);

		// Change the equipment at the specified slot and recalculate stats.
		void change_equip(int16_t slot);

		// Use the item from the player's inventory with the given id.
		void use_item(int32_t itemid);

		// Whether an item or stat request may go to the server now: the
		// player is alive unless allow_dead, no request waits on an answer,
		// and wait milliseconds have passed since the last request or answer.
		bool can_request(int32_t wait, bool allow_dead = false) const;

		// Send the request if it may go now, and note that it waits on its
		// answer. Return whether it went.
		bool request(OutPacket &&packet, int32_t wait, bool allow_dead = false);

		// Note the server answered, as an update with its first byte set does.
		void request_answered();

		// Forget a request waiting on its answer, as entering a field does.
		void clear_request();

		// End the attack now, held or not, and take up the state the player
		// is in.
		void end_attack();

		// Return if the player is attacking
		bool is_attacking() const;

		// Return whether the player is free to attack, weapon aside.
		bool can_attack() const;

		// Return whether the player can use a skill or not.
		SpecialMove::ForbidReason can_use(const SpecialMove &move) const;

		// Build a skill (0 for regular) or a point-blank whack from the player's stats.
		Attack prepare_attack(int32_t skill, bool whack = false) const;

		// Execute a rush movement
		void rush(double targetx);

		// Check whether the player is invincible
		bool is_invincible() const override;

		// Handle an attack to the player.
		MobAttackResult damage(const MobAttack &attack);

		// Apply a buff to the player
		void give_buff(Buff buff);

		// Cancel a buff.
		void cancel_buff(Buffstat::Id stat);

		// Return whether the buff is active.
		bool has_buff(Buffstat::Id stat) const;

		// Change a skill
		void change_skill(int32_t skill_id, int32_t level, int32_t masterlevel, int64_t expiration);

		// Put a skill on cooldown.
		void add_cooldown(int32_t skill_id, int32_t time);

		// Check if a skill is on cooldown.
		bool has_cooldown(int32_t skill_id) const;

		// Change the player's level, display the "level up" effect.
		void change_level(uint16_t level);

		// Change the player's job, display the job change effect.
		void change_job(uint16_t jobid);

		// Return the character's level
		uint16_t get_level() const override;

		// Return the character's level of a skill.
		int32_t get_skilllevel(int32_t skillid) const override;

		// Return the character's attacking speed.
		int8_t get_integer_attackspeed() const override;

		// Return the total ES_SPEED stat.
		int32_t get_walk_animation_speed() const override;

		// Returns the shoe's walking speed multiplier, from the total ES_SPEED stat.
		double get_walkspeed() const;

		// Returns the takeoff speed in pixels per tick, from the total ES_JUMP stat.
		double get_jumpforce() const;

		// Returns the flying force.
		float get_flyforce() const;

		// Return whether the player is underwater
		bool is_underwater() const;

		// Returns if a Keyaction is currently active.
		bool is_key_down(KeyAction::Id action) const;

		// Return a pointer to the ladder the player is on.
		Optional<const Ladder> get_ladder() const;

		// Change players position to the seat's position and stance to Char::State::SIT
		void set_seat(Optional<const Seat> seat);

		// Scale natural recovery by the map's info/recovery.
		void set_recovery_rate(double rate);

		// Sit down on the portable chair itemid where the player stands.
		void sit_on_chair(int32_t itemid);

		// Ask the server to let the player up from their portable chair.
		void leave_chair();

		// Get up from the portable chair once the server allows it.
		void stand_up();

		void set_chair(int32_t itemid) override;

		// Change players x-pos to the ladder x and change stance to Char::LADDER or Char::ROPE.
		void set_ladder(Optional<const Ladder> ladder);

		// Obtain a reference to the player's stats.
		CharStats &get_stats();

		// Obtain a reference to the player's stats.
		const CharStats &get_stats() const;

		// Obtain a reference to the player's inventory.
		Inventory &get_inventory();

		// Obtain a reference to the player's inventory.
		const Inventory &get_inventory() const;

		// Obtain a reference to the player's skills.
		SkillBook &get_skills();

		// Obtain a reference to the player's questlog.
		QuestLog &get_quests();

		// Obtain a reference to the player's telerock locations.
		TeleportRock &get_teleport_rock();

		// Obtain a reference to the player's monsterbook.
		MonsterBook &get_monsterbook();

		// Obtain a reference to the player's party.
		Party &get_party();
		const Party &get_party() const;

		// Obtain a reference to the player's buddy list.
		BuddyList &get_buddies();
		const BuddyList &get_buddies() const;

	private:
		// Natural recovery as v83 runs it (FUN_0083007f). MP heals every 10
		// seconds while alive. HP heals every 10 seconds, or 5 for a warrior
		// with Improved HP Recovery, while the player stands or sits without
		// moving or being alert; on a ladder or rope only with Endure, at its
		// interval. The map's info/recovery and a chair scale the amounts. A
		// portable chair heals its recoveryHP/recoveryMP instead. Each timer
		// restarts whenever its conditions fail.
		struct Recovery
		{
			uint16_t timer = 0;
			bool from_chair = false;

			// Advance the timer and return the amount due after interval
			// milliseconds, or 0. Like v83, the timer restarts when the source
			// switches between chair and natural.
			int16_t update(uint16_t interval, int16_t chair_amount, int16_t natural_amount);
		};

		static constexpr uint16_t RECOVERY_INTERVAL = 10000;
		static constexpr uint16_t WARRIOR_HP_RECOVERY_INTERVAL = 5000;
		static constexpr int16_t RECOVERY_HP = 10;
		static constexpr int16_t RECOVERY_MP = 3;
		static constexpr double CHAIR_RECOVERY_RATE = 1.5;
		static constexpr int32_t IMPROVED_HP_RECOVERY = 1000000;
		static constexpr int32_t WARRIOR_ENDURE = 1000002;
		static constexpr int32_t FIGHTER_IMPROVED_MP_RECOVERY = 1110000;
		static constexpr int32_t PAGE_IMPROVED_MP_RECOVERY = 1210000;
		static constexpr int32_t IMPROVED_MP_RECOVERY = 2000000;
		static constexpr int32_t ASSASSIN_ENDURE = 4100002;
		static constexpr int32_t BANDIT_ENDURE = 4200001;
		double recovery_rate = 1.0;
		Recovery hp_recovery;
		Recovery mp_recovery;
		int16_t chair_recovery_hp = 0;
		int16_t chair_recovery_mp = 0;
		// Whether the player asked to get off their chair and waits for the server.
		bool leaving_chair = false;

		void update_recovery();

		// What the player's skills add to each natural HP or MP heal
		// (FUN_00652fa8, FUN_0065308e).
		int16_t get_recovery_hp_bonus() const;
		int16_t get_recovery_mp_bonus() const;

		// Endure's milliseconds between HP heals on a ladder or rope, or 0
		// without it (FUN_0065321e).
		uint16_t get_endure_interval() const;

		// Die when HP runs out, and get back up when it returns.
		void update_death();

		// Steps since the player died; the revive window opens after 2.2 seconds.
		static constexpr uint16_t REVIVE_DELAY = 2200 / Constants::TIMESTEP;
		uint16_t death_ticks = 0;

		CharStats stats;
		Inventory inventory;
		SkillBook skillbook;
		QuestLog questlog;
		TeleportRock teleportrock;
		MonsterBook monsterbook;
		Party party;
		BuddyList buddies;

		EnumMap<Buffstat::Id, Buff> buffs;
		ActiveBuffs active_buffs;
		PassiveBuffs passive_buffs;

		std::unordered_map<int32_t, int32_t> cooldowns;

		ExclRequest excl;

		std::map<KeyAction::Id, bool> keysdown;

		move_path::Recorder path;

		uint8_t get_stancebyte() const;

		// Where the player is and how it moves, as path element `command`.
		Movement current_move(uint8_t command) const;

		Randomizer randomizer;

		Optional<const Ladder> ladder;

		bool underwater;
	};
}