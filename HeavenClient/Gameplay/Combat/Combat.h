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

#include "FinalAttack.h"
#include "MonsterMagnet.h"
#include "RegularAttack.h"
#include "ShotPath.h"
#include "ShotRain.h"
#include "SpellArea.h"
#include "Teleport.h"

#include "../MapleMap/MapChars.h"
#include "../MapleMap/MapDrops.h"
#include "../MapleMap/MapInfo.h"
#include "../MapleMap/MapMobs.h"
#include "../MapleMap/MapPortals.h"
#include "../MapleMap/MapReactors.h"
#include "../MapleMap/MapSummons.h"
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
		Combat(Player &player, MapChars &chars, MapMobs &mobs, MapReactors &reactors, MapSummons &summons,
			   MapDrops &drops, const Physics &physics, const MapInfo &mapinfo, const MapPortals &portals);

		// Load the charge gauge, once the NX files are open.
		static void init();

		// Draw bullets, damage numbers etc.
		void draw(double viewx, double viewy, float alpha) const;

		// Poll attacks, damage effects, etc.
		void update();

		// Make the player use a special move. Returns whether it went off.
		bool use_move(int32_t move_id);

		// Let go of the skill held down, as its key comes up.
		void release_prepare(int32_t move_id);

		// Whether a skill is held down.
		bool is_preparing() const;

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

		// Ask to use Flash Jump, which the map must have `allowed` and the
		// player be `airborne` for. Whether it was asked.
		bool use_flash_jump(int32_t move_id, bool allowed, bool airborne);

		// Ask to use Wings, which the map must have `allowed` and the player
		// `may` cast now; it says nothing when not. Whether it was asked.
		bool use_wings(int32_t move_id, bool allowed, bool may);

		// Add an attack to the attack queue.
		void push_attack(const AttackResult &attack);

		// With a full gauge, Energy Charge hits the mob `oid` touching the player, once a second at most
		// (FUN_0095f135 from 0x94b3aa).
		void body_attack(int32_t oid);

		// Show a buff effect.
		void show_buff(int32_t cid, int32_t skillid, int8_t level);

		// Show another user starting to prepare a skill (0x980a81).
		void show_prepare(int32_t cid, int32_t skillid, uint8_t level, uint16_t action_and_facing, uint8_t speed);

		// Show another user letting go of a held skill (0x980bf5).
		void cancel_prepare(int32_t cid, int32_t skillid);

		// Show a buff effect.
		void show_player_buff(int32_t skillid);

		// Show a party member's skill reaching another user, or the player
		// (user effect 2, 0x937b83).
		void show_affected(int32_t cid, int32_t skillid);
		void show_player_affected(int32_t skillid);

		// Show Recoil Shot's effect on another user whose path shows its push
		// (0x980104).
		void show_recoil_shot(Char &user);

		// Show a skill's `special` animation on a user or the player (0x936528): the skill that
		// dodged a hit, Guardian or Shadow Shifter, or Dragon Blood drinking HP (user effect 5).
		void show_special(int32_t cid, int32_t skillid);
		void show_player_special(int32_t skillid);

		// Show Monster Magnet catching a mob, or failing to, over it
		// (0x66d6b9, 0x438eb6).
		void show_catch(int32_t oid, bool success);

		// Show the catch item `itemid` thrown at a mob catching it, or failing to: its ItemEff, or that
		// effect's `fail`, over the mob; the Element Rock's sound too (0x6692c8, 0x66934f).
		void show_item_catch(int32_t oid, int32_t itemid, bool success);

		// Show another user throwing a grenade from `position`, held for
		// `charge` ms (0x983bc1).
		void show_throw(int32_t cid, Point<int16_t> position, int32_t charge, int32_t skillid);

		// Drop the grenades in flight, as on leaving the map.
		void clear_grenades();

		// Drop a Final Attack still to come, as on leaving the map: v83
		// makes the user anew in each field.
		void clear_final_attack();

		// Show another user's summon attacking: its animation, then each
		// mob's hit as it gets there (FUN_007a6882). `hits` are the mobs
		// and their damage.
		void show_summon_attack(int32_t oid, uint8_t action, const std::vector<std::pair<int32_t, int32_t>> &hits);

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

		// Attacking or casting ends Dash first (0x9678e1).
		void end_dash() const;

		// Use a skill that doesn't attack. Whether it could ask the server.
		bool request_skill(const SpecialMove &move);

		// Whether a move may go with the player on a mount, or getting on or off one (FUN_00966f7a). Monster
		// Rider gets off a tamed monster instead.
		bool ride_allows(int32_t move_id);

		// Summon in front of the player, if there is room (FUN_0096b140).
		// Whether it could ask the server.
		bool request_summon(const SpecialMove &move, int32_t level);

		// Open a Mystic Door where the player stands, unless the map or a
		// portal forbids it (FUN_00969bf9).
		bool request_door(const SpecialMove &move, int32_t level);

		// What one of a summon's attacks is like: its
		// `summon/attack1/info`.
		struct SummonInfo
		{
			uint8_t mobcount;
			// How far it reaches to either side.
			int16_t reach;
			// Its box, from where it is, facing left; empty if none.
			Rectangle<int16_t> area;
			int16_t attackafter;
			// Its ball, which flies 600 ms per `bulletspeed` px; the
			// Octopi's only.
			std::optional<Animation> ball;
			int16_t bulletspeed;
			// Where the ball leaves it, facing left.
			Point<int16_t> sp;
			Animation hit;
		};

		const SummonInfo &get_summon_info(int32_t skill);

		// The player's summons attack when they may (FUN_00765047).
		void update_summons();
		// The player's Beholder heals them if it may (FUN_007a5ab4), and
		// whether it did.
		bool beholder_heal(int32_t oid, Summon &beholder);
		// The player's Beholder buffs a stat of theirs (FUN_007a5c99).
		void beholder_hex(int32_t oid, Summon &beholder);

		// A summon's hit landing on a mob.
		struct SummonHit
		{
			int32_t oid;
			int32_t damage;
			bool toleft;
			int32_t skill;
		};

		void apply_summon_hit(const SummonHit &hit);

		// What the skill's packet carries besides it, or nothing if it isn't
		// sent: Shadow Stars without its stars, or Resurrection with no one
		// to raise.
		std::optional<UseSkillPacket::Reach> find_reach(int32_t skill, int32_t level);

		// The party members a skill reaches in `range`, a bit for each in
		// `mask`, and how many (0x96db3f).
		int32_t find_party(int32_t skill, Rectangle<int16_t> range, uint8_t &mask) const;

		// Tell the server whom in the party Heal heals (0x96a69d), and how
		// many.
		int32_t request_heal();

		// The undead in `range` that Heal hurts, at most `count`, in the
		// order they entered the map (FUN_00678760).
		std::vector<int32_t> find_undead(Rectangle<int16_t> range, uint8_t count) const;

		// A Final Attack in this stance, after the skill `trigger`.
		struct Follow
		{
			Stance::Id stance;
			int32_t trigger;
		};

		// Use the move. A held skill hits with its `charge`, and its shots
		// while held keep the body's action (`held`). A Final Attack
		// (`follow`) cuts short the action under way with its own.
		// Assassinate's second strike (`again`) hits only that mob. Whether
		// it went out.
		bool apply_move(const SpecialMove &move, int32_t charge = 0, bool held = false,
						const Follow *follow = nullptr, std::optional<int32_t> again = std::nullopt);

		// Meso Explosion: set off the mesos in front of the player, each
		// hurting the mobs around it (FUN_0096b3fb). Whether it went off.
		bool set_off_mesos(const SpecialMove &move);

		// After a skill attack, see whether one of the Final Attacks it may
		// set off comes up, and when it follows (0x957830).
		void roll_final_attack(int32_t trigger);

		// Follow up with the Final Attack that came up, once it's time
		// (0x96cd3e).
		void update_final_attack();
		// Strike again with Assassinate when due, and dash forward, if the
		// dash finds somewhere to land (FUN_0096ce79, FUN_0096cb93).
		void update_second_strike();

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

		// Start holding a skill down (FUN_0096a86e). Whether it started.
		bool start_prepare(const Skill &skill);

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

		// Rain what a shot rains once it hits, `hit` ms from now: over the
		// user, around where it hits its first mob (`first`), or over each
		// mob it hits (`bodies`), as in v83 (0x95496f, 0x9821f7).
		void rain_shot(int32_t skill, Rectangle<int16_t> lt_rb, Point<int16_t> user, bool toleft, int64_t hit,
					   Point<int16_t> first, const std::vector<Rectangle<int16_t>> &bodies);

		// Drop a skill's rain over an area from `hit` ms from now
		// (FUN_0043612f).
		void start_rain(const Skill &skill, Rectangle<int16_t> area, bool toleft, int64_t hit);

		// Set the ground across an area on fire `hit` ms from now, for
		// `length` ms (FUN_00436ce9).
		void burn(const Skill &skill, Rectangle<int16_t> area, int64_t hit, int32_t length);

		// Leave what an area spell leaves over the field as it's cast, with
		// its lt/rb, how long it was charged, and where its user stands and
		// faces (0x956b4e, 0x982b33).
		void cast_area(int32_t skill, Rectangle<int16_t> lt_rb, int32_t charge, Point<int16_t> user, bool toleft);

		void update_rains();

		void draw_rains(double viewx, double viewy, float alpha) const;

		// Swing a basic attack at a reactor in reach instead, if there is
		// one. Returns whether it did.
		bool swing_at_reactor();

		// The melee stance a regular ranged attack whacks a mob right in
		// front of the player with, or NONE to shoot. Either way, `reach` is
		// how far in front of the player the whack reaches.
		Stance::Id find_whack(const Attack &attack, int16_t &reach) const;

		// How far in front of the user a whack in this stance reaches.
		static int16_t get_whack_reach(const Char &user, Stance::Id stance);

		// How far in front of the player their shot with this attack reaches.
		int16_t get_shot_range(const Attack &attack) const;

		// The mob a shot from `origin` toward `end_x` hits: the first one in
		// its path, as it widens away from the player, or only mob `only`.
		std::vector<int32_t> find_shot_target(Point<int16_t> origin, int16_t end_x, int32_t only = 0) const;

		// The mobs Chain Lightning hits, starting with `first`.
		std::vector<int32_t> find_chain(int32_t first, uint8_t count, bool toleft) const;

		// The platforms a Teleport might land on.
		std::vector<teleport::Floor> find_floors() const;

		// The platforms a shot from `from` to `to` could run into.
		std::vector<shot_path::Platform> find_platforms(Point<int16_t> from, Point<int16_t> to) const;

		std::vector<int32_t>
		find_targets(MapObjects *objs, Rectangle<int16_t> range, Point<int16_t> origin, uint8_t objcount,
					 bool closest_first) const;

		void apply_use_movement(const SpecialMove &move);

		void apply_result_movement(const SpecialMove &move, const AttackResult &result);

		void apply_rush(const AttackResult &result);

		void apply_bullet_effect(const Bullet &bullet);

		void apply_damage_effect(const DamageEffect &effect);

		// Another user's Meso Explosion as its mobs show it: each line its
		// own number, but their fixed damage, or 0 if it can't hurt them
		// (FUN_009803ab).
		AttackResult show_explosion(AttackResult attack) const;

		// The area an attack covers, from the attacker's position and facing.
		static Rectangle<int16_t> place_range(Point<int16_t> origin, Rectangle<int16_t> range, float hrange,
											  bool toleft);

		// A shot starts `reach` px in front of the user, covers `shot_area`
		// and flies to `shot_end`, as far as it reaches or where a platform
		// stops it.
		void extract_effects(const Char &user, const SpecialMove &move, const AttackResult &result,
							 Rectangle<int16_t> range, int16_t reach, Rectangle<int16_t> shot_area,
							 Point<int16_t> shot_end);

		// Show another user's spell: its balls, and its hits as they get
		// there (FUN_0098250d).
		void show_remote_spell(const Char &user, const SpecialMove &move, const AttackResult &result,
							   Rectangle<int16_t> range);

		// Show another user's shot: its bullets, and its hits as they would
		// get there (FUN_00981211).
		void show_remote_shot(const OtherChar &user, const SpecialMove &move, const AttackResult &result,
							  Rectangle<int16_t> range);

		// Draw Chain Lightning's bolt from the user through its targets, each
		// part as its target is hit (`staggered`) or all at once.
		void draw_chain(const Char &user, const SpecialMove &move, const AttackResult &result, bool staggered);

		// Each target with a counter hurts the player back; v83 shows its
		// own roll over the head and sends nothing (FUN_00953595): the
		// server takes the HP.
		void show_counters(const AttackResult &result);

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
		MapSummons &summons;
		MapDrops &drops;
		const Physics &physics;
		const MapInfo &mapinfo;
		const MapPortals &portals;

		std::unordered_map<int32_t, Skill> skills;
		RegularAttack regularattack;

		TimedQueue<AttackResult> attackresults;
		TimedQueue<Bullet> bulleteffects;
		TimedQueue<DamageEffect> damageeffects;
		TimedQueue<SummonHit> summonhits;
		std::unordered_map<int32_t, SummonInfo> summoninfos;

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

		// A shot's rain, dropping its sprites a few at a time.
		struct Rain
		{
			int32_t skill;
			Rectangle<int16_t> area;
			bool toleft;
			shot_rain::Schedule schedule;
		};

		struct RainDrop
		{
			Animation animation;
			Point<int16_t> start;
			Point<int16_t> fall;
			bool flip;
			int64_t launch;
			int32_t flight;
			float opacity;
		};

		// A blast's sprites, landing a wave at a time further out.
		struct Blast
		{
			int32_t skill;
			shot_rain::Schedule schedule;
			spell_area::Spread spread;
		};

		// One of the flames, or tiles, on the ground.
		struct Flame
		{
			Animation animation;
			Point<int16_t> position;
			int64_t start;
			int32_t fade;
			float opacity;
			int32_t length;
		};

		std::list<Rain> rains;
		std::list<RainDrop> raindrops;
		std::list<Blast> blasts;
		std::list<Flame> flames;

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

		// A Final Attack that came up: what set it off, with which weapon
		// type, and when it follows.
		struct FinalAttack
		{
			int32_t trigger;
			int32_t skill;
			int32_t weapon;
			int64_t at;
		};

		std::optional<FinalAttack> pending_final;

		// Assassinate's second strike: on which mob, with which weapon
		// type, and when (0x957977).
		struct SecondStrike
		{
			int32_t oid;
			int32_t weapon;
			int64_t at;
		};

		std::optional<SecondStrike> second_strike;

		// Game time in ms, for the shots of the skills that keep firing.
		int64_t now = 0;
		// When Hurricane and Rapid Fire last shot. They outlast a hold, so
		// letting go and pressing again doesn't shoot sooner (0x948ee5).
		int64_t last_hurricane = INT32_MIN;
		int64_t last_rapidfire = INT32_MIN;
		// When Energy Charge may hit a touching mob again (0x95f2e6).
		int64_t next_body_attack = 0;

		static Texture gauge_bar[2];
		static Texture gauge_fill[2];
		static Texture gauge_graduation[2];
		static Animation catch_success;
		static Animation catch_fail;
		static Animation grenade_ball;
	};
}