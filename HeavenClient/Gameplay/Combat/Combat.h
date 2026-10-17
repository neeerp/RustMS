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

#include "MonsterMagnet.h"
#include "RegularAttack.h"
#include "ShotPath.h"
#include "Teleport.h"

#include "../MapleMap/MapChars.h"
#include "../MapleMap/MapInfo.h"
#include "../MapleMap/MapMobs.h"
#include "../MapleMap/MapReactors.h"
#include "../Physics/Physics.h"

#include "../../Character/Player.h"
#include "../../Graphics/EffectLayer.h"
#include "../../Net/Packets/AttackAndSkillPackets.h"
#include "../../Template/TimedQueue.h"

namespace ms
{
	class Combat
	{
	public:
		Combat(Player &player, MapChars &chars, MapMobs &mobs, MapReactors &reactors, const Physics &physics,
			   const MapInfo &mapinfo);

		// Load the charge gauge, once the NX files are open.
		static void init();

		// Draw bullets, damage numbers etc.
		void draw(double viewx, double viewy, float alpha) const;

		// Poll attacks, damage effects, etc.
		void update();

		// Make the player use a special move
		void use_move(int32_t move_id);

		// Let go of the skill held down, as its key comes up.
		void release_prepare(int32_t move_id);

		// Let go of the skill held down, if any, and use it, as when the
		// window loses focus (FUN_0095bedf).
		void finish_prepare();

		// The player was hit for `damage`: a hit lets go of a skill that keeps
		// firing (0x95924f).
		void interrupt_prepare(int32_t damage);

		// Stop holding a skill without using it, as on leaving the map
		// (0x9785bd).
		void clear_prepare();

		// Make the player use a Teleport skill, if the stage `allowed` it.
		// Where the player should be moved to, if the skill was used.
		std::optional<teleport::Spot> use_teleport(int32_t move_id, bool allowed);

		// Add an attack to the attack queue.
		void push_attack(const AttackResult &attack);

		// Show a buff effect.
		void show_buff(int32_t cid, int32_t skillid, int8_t level);

		// Show another user starting to prepare a skill (0x980a81).
		void show_prepare(int32_t cid, int32_t skillid, uint8_t level, uint16_t action_and_facing, uint8_t speed);

		// Show another user letting go of a held skill (0x980bf5).
		void cancel_prepare(int32_t cid, int32_t skillid);

		// Show a buff effect.
		void show_player_buff(int32_t skillid);

		// Show Monster Magnet catching a mob, or failing to, over it
		// (0x66d6b9, 0x438eb6).
		void show_catch(int32_t oid, bool success);

		// Show another user throwing a grenade from `position`, held for
		// `charge` ms (0x983bc1).
		void show_throw(int32_t cid, Point<int16_t> position, int32_t charge, int32_t skillid);

		// Drop the grenades in flight, as on leaving the map.
		void clear_grenades();

	private:
		struct DamageEffect
		{
			AttackUser user;
			DamageNumber number;
			int32_t damage;
			bool toleft;
			int32_t target_oid;
			int32_t move_id;
		};

		void apply_attack(const AttackResult &attack);

		// Whether the player may use the move now, saying why not if not.
		bool can_use_move(const SpecialMove &move) const;

		// Use a skill that doesn't attack. Whether it could ask the server.
		bool request_skill(const SpecialMove &move);

		// What the skill's packet carries besides it, or nothing if it isn't
		// sent: Shadow Stars without its stars, or Resurrection with no one
		// to raise.
		std::optional<UseSkillPacket::Reach> find_reach(int32_t skill, int32_t level);

		// Use the move. A held skill hits with its `charge`, and its shots
		// while held keep the body's action (`held`).
		void apply_move(const SpecialMove &move, int32_t charge = 0, bool held = false);

		// Let go of Monster Magnet, held for `charge` ms: it catches the mobs
		// in front of the player (0x96c215). Whether it was used.
		bool use_magnet(const Skill &skill, int32_t level, int32_t charge);

		// Throw a grenade for the user `owner` from `position`, harder the
		// longer it was held (0x9420c6).
		void throw_grenade(int32_t owner, int32_t skill, int32_t charge, Point<int16_t> position, bool toleft);

		// Move the grenades, and set off the ones that hit something
		// (0x58fc1e).
		void update_grenades();

		// A grenade goes off: the player's own hits the mobs around it
		// (0x950921), anyone else's only shows its effect (0x936528).
		void explode(int32_t owner, int32_t skill, Point<int16_t> position);

		// Start holding a skill down (FUN_0096a86e).
		void start_prepare(const Skill &skill);

		// Hold the skill another step: its effects, and its next shot.
		void update_prepare();

		// Set off a timed skill, its action over (0x96d01e).
		void complete_prepare();

		// Fire a skill that keeps firing, if it can, or let it go.
		void shoot_prepare(const Skill &skill);

		// Hold the skills other users hold another step: their effects turn
		// to the keydown ones in time (0x980328).
		void update_remote_prepares();

		// Stop showing another user's prepare: its action and its effect.
		void end_remote_prepare(OtherChar &user);

		void draw_gauge(double viewx, double viewy, float alpha) const;

		// The melee stance a regular ranged attack whacks a mob right in
		// front of the player with, or NONE to shoot. Either way, `reach` is
		// how far in front of the player the whack reaches.
		Stance::Id find_whack(const Attack &attack, int16_t &reach) const;

		// How far in front of the user a whack in this stance reaches.
		static int16_t get_whack_reach(const Char &user, Stance::Id stance);

		// How far in front of the player their shot with this attack reaches.
		int16_t get_shot_range(const Attack &attack) const;

		// The mob a shot from `origin` toward `end_x` hits: the first one in
		// its path, as it widens away from the player.
		std::vector<int32_t> find_shot_target(Point<int16_t> origin, int16_t end_x) const;

		// The mobs Chain Lightning hits, starting with `first`.
		std::vector<int32_t> find_chain(int32_t first, uint8_t count, bool toleft) const;

		// The platforms a Teleport might land on.
		std::vector<teleport::Floor> find_floors() const;

		// The platforms a shot from `from` to `to` could run into.
		std::vector<shot_path::Platform> find_platforms(Point<int16_t> from, Point<int16_t> to) const;

		std::vector<int32_t>
		find_targets(MapObjects *objs, Rectangle<int16_t> range, Point<int16_t> origin, uint8_t objcount,
					 bool use_mobs, bool closest_first) const;

		void apply_use_movement(const SpecialMove &move);

		void apply_result_movement(const SpecialMove &move, const AttackResult &result);

		void apply_rush(const AttackResult &result);

		void apply_bullet_effect(const Bullet &bullet);

		void apply_damage_effect(const DamageEffect &effect);

		// The area an attack covers, from the attacker's position and facing.
		static Rectangle<int16_t> place_range(Point<int16_t> origin, Rectangle<int16_t> range, float hrange,
											  bool toleft);

		// A shot starts `reach` px in front of the user and flies to `shot_end`,
		// as far as it reaches or where a platform stops it.
		void extract_effects(const Char &user, const SpecialMove &move, const AttackResult &result,
							 Rectangle<int16_t> range, int16_t reach, Point<int16_t> shot_end);

		// Show another user's spell: its balls, and its hits as they get
		// there (FUN_0098250d).
		void show_remote_spell(const Char &user, const SpecialMove &move, const AttackResult &result,
							   Rectangle<int16_t> range);

		// Draw Chain Lightning's bolt from the user through its targets, each
		// part as its target is hit (`staggered`) or all at once.
		void draw_chain(const Char &user, const SpecialMove &move, const AttackResult &result, bool staggered);

		// Tell the targets how many hits are coming (sign 1) or cancel them (sign -1),
		// so a mob the server kills waits for the hits before dying.
		void expect_hits(const AttackResult &result, int32_t sign);

		std::vector<DamageNumber> place_numbers(const std::vector<std::pair<int32_t, bool>> &damagelines, int32_t move_id);

		const SpecialMove &get_move(int32_t move_id);

		const Skill &get_skill(int32_t skill_id);

		Player &player;
		MapChars &chars;
		MapMobs &mobs;
		MapReactors &reactors;
		const Physics &physics;
		const MapInfo &mapinfo;

		std::unordered_map<int32_t, Skill> skills;
		RegularAttack regularattack;

		TimedQueue<AttackResult> attackresults;
		TimedQueue<Bullet> bulleteffects;
		TimedQueue<DamageEffect> damageeffects;

		// A mob Monster Magnet reached, caught or not, for the user where
		// they stood.
		struct MagnetCatch
		{
			int32_t oid;
			bool success;
			Point<int16_t> user;
		};

		TimedQueue<MagnetCatch> magnetcatches;
		// Effects fixed in the map: Monster Magnet's catches, and grenades
		// going off.
		EffectLayer fieldeffects;

		// A grenade in flight, thrown by the user `owner`.
		struct Grenade
		{
			int32_t owner;
			int32_t skill;
			PhysicsObject phobj;
			Animation ball;
		};

		std::list<Grenade> grenades;

		// When Monster Magnet and the throws may be held again, by skill
		// (user+0x3158).
		std::unordered_map<int32_t, int64_t> reuse_times;

		std::list<Bullet> bullets;
		std::list<DamageNumber> damagenumbers;

		// A skill held down: how long, in ms, and whether its effect has
		// turned to the keydown one.
		struct Preparing
		{
			int32_t skill;
			int32_t level;
			int32_t held = 0;
			bool forced = false;
			bool keydown = false;
			// How long a timed skill prepares for.
			int32_t duration = 0;
		};

		std::optional<Preparing> preparing;

		// A skill another user holds down, by their id: how long, in ms,
		// whether its effect has turned to the keydown one, and whether it
		// took up their action.
		struct RemotePrepare
		{
			int32_t skill;
			int32_t held = 0;
			bool keydown = false;
			bool forced = false;
		};

		std::unordered_map<int32_t, RemotePrepare> remote_prepares;

		// Game time in ms, for the shots of the skills that keep firing.
		int64_t now = 0;
		// When Hurricane and Rapid Fire last shot. They outlast a hold, so
		// letting go and pressing again doesn't shoot sooner (0x948ee5).
		int64_t last_hurricane = INT32_MIN;
		int64_t last_rapidfire = INT32_MIN;

		static Texture gauge_bar[2];
		static Texture gauge_fill[2];
		static Texture gauge_graduation[2];
		static Animation catch_success;
		static Animation catch_fail;
		static Animation grenade_ball;
	};
}