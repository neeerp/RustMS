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

#include <map>
#include <optional>
#include <string>
#include <vector>

#include <cstddef>

#include <cstdint>

#include "MapObject.h"
#include "MobAttackRules.h"
#include "MobHurts.h"

#include "../Movement.h"
#include "../MovePath.h"

#include "../Combat/Attack.h"
#include "../Combat/Bullet.h"
#include "../Combat/MagicDamage.h"

#include "../../Audio/Audio.h"
#include "../../Character/Beacon.h"
#include "../../Net/Handlers/Helpers/MobStatParser.h"
#include "../../Graphics/EffectLayer.h"
#include "../../Graphics/Geometry.h"
#include "../../Graphics/Texture.h"
#include "../../Util/Randomizer.h"
#include "../../Util/TimedBool.h"

namespace ms
{
	class Mob : public MapObject
	{
	public:
		static const size_t NUM_STANCES = 6;
		// The action of attack1; attack2 and on follow (FUN_0066b6fc).
		static constexpr int8_t ATTACK1 = 12;

		enum Stance : uint8_t
		{
			MOVE = 2,
			STAND = 4,
			JUMP = 6,
			HIT = 8,
			DIE = 10,
			// What v83 (FUN_005cc410) sends for a mob that walks with its
			// chase animation.
			CHASE = 32
		};

		static std::string nameof(Stance stance)
		{
			static const std::string stancenames[NUM_STANCES] =
					{
							"move", "stand", "jump", "hit1", "die1", "fly"
					};

			size_t index = (stance - 1) / 2;

			return stancenames[index];
		}

		static uint8_t value_of(Stance stance, bool flip)
		{
			return flip ? stance : stance + 1;
		}

		// Construct a mob by combining data from game files with data sent by the server.
		Mob(int32_t oid, int32_t mobid, int8_t mode, int8_t stance, uint16_t fhid, uint16_t homefh, bool newspawn,
			int8_t team, Point<int16_t> position, const std::vector<MobStatParser::Status> &statuses,
			const Physics &physics);

		// Draw the mob
		void draw(double viewx, double viewy, float alpha) const override;

		// Update movement and animations.
		int8_t update(const Physics &physics) override;

		// Change this mob's control mode:
		// 0 - no control, 1 - control, 2 - aggro
		void set_control(int8_t mode);

		// The local player, as a mob chases and attacks them.
		struct Target
		{
			int32_t id;
			Point<int16_t> position;
			// Where the player's body is, and whether they stand on a
			// foothold.
			mob_attack::Box body;
			bool onground;
			bool dead;
			// Dark Sight keeps physical attacks off.
			bool darksight;
			// The mob the player's Homing Beacon marked.
			beacon::Mark guided;
		};

		// Tell the mob where the local player is, whom it chases with aggro
		// and whom its attacks hit.
		void set_target(const Target &target);

		// The player's Puppet `oid` came at `position`: a mob this client
		// moves that chases someone turns on it, and keeps to it until it
		// goes (FUN_00679847, 0x9bbd88).
		void aggro_puppet(int32_t oid, Point<int16_t> position);

		// Puppet `oid` went: a mob that chased it chases no one.
		void release_puppet(int32_t oid);

		// Roll what its touch does to a Puppet of skill level `level`.
		int32_t roll_puppet_damage(int32_t level) const;

		// The server got the move with this id (FUN_0066c23b), saying whether
		// the mob may attack or cast next, how much MP it has, and which
		// skill at which level it may cast (0 for none).
		void acknowledge(int16_t moveid, bool nextattack, int16_t mp, uint8_t skill = 0, uint8_t skilllevel = 0);

		// Show a summoned mob's appearance, Effect/Summon.img/<type>, once
		// `delay` ms have passed.
		void show_summon(int8_t type, int32_t delay);

		// Queue the controller's path to play (0xEF), and what it did where
		// the path starts: `action` (-1 for nothing), facing left if `left`.
		void send_movement(Point<int16_t> start, std::vector<Movement> &&movements, int8_t action, bool left,
						   int32_t skilldata);

		// A hit on the local player from one of the mob's attacks, and what
		// shows on them.
		struct Landed
		{
			MobAttack attack;
			Animation hit;
		};

		// The local player's hits from this mob's attacks since last asked.
		std::vector<Landed> take_attacks();

		// Kill the mob with the appropriate type (FUN_0067961d): 0 makes it
		// inactive, 1 plays its death, 3 its skill1, and 2 or 4 and up blow
		// it up with attack1. The death waits for any hits still on their
		// way.
		void kill(int8_t killtype);

		// Note hits that will land on this mob once their attack connects;
		// a negative count cancels hits that won't land after all.
		void expect_hits(int32_t count);

		// Display the hp percentage above the mob.
		// Use the playerlevel to determine color of nametag.
		void show_hp(int8_t percentage, uint16_t playerlevel);

		// Show a skill's hit effect where its `pos` puts it, given where the
		// attack reached.
		void show_effect(const Animation &animation, int8_t pos, int8_t z, bool flip,
						 const Rectangle<int16_t> &range);

		// Take on the statuses a spawn or MOB_STAT_SET gives (FUN_0078b0b1).
		// With `effects`, each new one's skill plays its effect on the mob.
		void set_statuses(const std::vector<MobStatParser::Status> &statuses, bool effects);

		// Lose the statuses MOB_STAT_RESET names.
		void reset_statuses(const BuffParser::Flags &flags);

		// Show a weapon's hit spark where the attack meets the mob's body.
		void show_hit(const Animation &animation, const Rectangle<int16_t> &range, bool flip);

		// Calculate the damage to this mob with the specified attack.
		std::vector<std::pair<int32_t, bool>> calculate_damage(const Attack &attack);

		// Meso Explosion's lines on the mob from drops of these `mesos`,
		// with the skill's `x` (FUN_00791fbc).
		std::vector<int32_t> explode(int32_t x, const std::vector<int32_t> &mesos) const;

		// What a number of Meso Explosion's shows on the mob: its fixed
		// damage for a hit, or 0 if the skill can't hurt it.
		int32_t shown_explosion(int32_t number) const;

		// Apply damage to the mob.
		void apply_damage(int32_t damage, bool toleft);

		// Monster Magnet caught the mob for the user at `user`: if this
		// client moves it, it flinches toward them, is pulled in, and
		// chases them (0x66b6fc, 0x66b562).
		void catch_by(Point<int16_t> user);

		// Create a touch damage attack to the player
		MobAttack create_touch_attack() const;

		// Check if touching this mob hurts: `bodyAttack` without `notAttack`.
		bool deals_touch_damage() const;

		// Check if this mob collides with the specified rectangle.
		bool is_in_range(const Rectangle<int16_t> &range) const;

		// Check if this mob is still alive.
		bool is_alive() const;
		// Its template id, as Mob.wz names it.
		int32_t get_id() const;

		// Whether a Teleport can't take the player past this mob.
		bool blocks_teleport() const;

		// Whether Monster Magnet can't catch the mob.
		bool is_boss() const;

		// Whether Heal hurts the mob.
		bool is_undead() const;

		// Whether the mob walks or flies.
		bool can_move() const;

		// Return the head position.
		Point<int16_t> get_head_position() const;

		// The numbers Poison, Venom and Ambush showed since last asked.
		std::vector<int32_t> take_hurts();

		// Show a number no attack made: damage, a heal if negative, or a
		// miss (FUN_006691d3).
		void show_damage(int32_t damage);

		// Show what another player's Power Guard (`physical`) or Mana
		// Reflection sent back at it: `percent` of their `damage`, a tenth
		// of its HP at most for Power Guard (FUN_009832e3).
		void show_reflect(uint8_t percent, int32_t damage, bool physical);

		// What its counter hurts the player with for one of their attacks,
		// a spell's (MCounter) or the rest (PCounter), or 0 (FUN_007937e3).
		int32_t counter(bool magic) const;

		// Return the body's bounds on the map, turned the way the mob faces.
		Rectangle<int16_t> get_body() const;

	private:
		// The stance and facing a stance byte stands for.
		Stance stance_of(uint8_t stancebyte, bool &left) const;

		// Show the stance a path gives.
		void show_stance(uint8_t stancebyte);

		// Show a stance, facing right if `right`.
		void show_stance(Stance newstance, bool right);

		// Set the stance the AI moves in.
		void set_body_stance(Stance newstance);

		// Where the AI has the mob.
		Point<int16_t> body_position() const;

		// Start the AI where the shown paths end (FUN_009bbd3d).
		void seat_body(const Physics &physics);

		// Step the AI ahead until it has a path to send, then send it with
		// what the mob did (`action`, -1 for nothing) and play it
		// (FUN_0066b6fc).
		void plan(const Physics &physics, int8_t action = -1, bool left = false, int32_t skilldata = 0);

		// Pick an attack that reaches the player, if any, and its facing
		// and skill data (FUN_0066a517).
		struct Pick
		{
			uint8_t index;
			bool left;
			int32_t skilldata;
		};

		std::optional<Pick> pick_attack(const Physics &physics);

		// Attack where the shown path is, hold the AI still while it plays,
		// and send it (FUN_0066b6fc).
		void attack(const Physics &physics, const Pick &pick);

		// Play attack `index` from (x, y) and queue what it hits
		// (FUN_0066d9c0).
		void do_attack(const Physics &physics, uint8_t index, bool left, int32_t skilldata, Point<int16_t> at);

		// Land the attacks that are due on the local player, and fly the
		// balls on (FUN_00666362).
		void update_attacks();

		// Cast the skill the server let it, if it has it, from where the
		// shown path is, and send it (FUN_0066e7af).
		bool cast(const Physics &physics);

		// Play a skill animation and queue the skill's effect at its level
		// (FUN_0066e7af, FUN_0066be61). Whether the mob has the skill.
		bool do_cast(uint8_t skill, uint8_t skilllevel, bool left);

		// What one of the mob's attacks does to the player.
		MobAttack make_attack(uint8_t index, bool left) const;

		// What its body does to the player, touching or not.
		MobAttack create_touch_attack_base() const;

		// The value of a status on the mob, or 0.
		int32_t status_value(uint8_t stat) const;

		// The ground within 100 px up or down of (x, y), if any.
		std::optional<int16_t> ground_below(const Physics &physics, int16_t x, int16_t y) const;

		// One step of the AI and its physics, recorded into the path.
		void step_body(const Physics &physics);

		// Stop what is shown at the next foothold, flinch from there, and
		// send the path at once.
		void flinch_body(const Physics &physics);

		// Start the death animation.
		void apply_death();

		// Some bosses shake the view as they die (FUN_00663995).
		void tremble_on_death() const;

		// Take one 90 ms step of a ground mob's AI.
		void think(const Physics &physics);

		// Chase the local player for 180 to 240 s, deciding at once.
		void chase();

		// Set the walking and flying speeds, at chaseSpeed if chasing.
		void update_shoe(bool chasing);

		// Whether a status holds the mob where it stands.
		bool held() const;

		// Show a status's skill on the mob: its `mob` effect once, or for
		// as long as the status lasts if it repeats, and its `mob0` for as
		// long as the status lasts.
		void show_status(int32_t reason, bool effects);

		// Show mob `mobid`'s look in place of its own, or its own again
		// with 0.
		void show_look(int32_t mobid);

		// Take one 90 ms step of a flying mob's AI.
		void think_fly(const Physics &physics);

		// Pick where a flier heads next.
		void pick_waypoint(const Physics &physics);

		// Steer a flier left (-1) or right (1), and glide (-1) or dive (0).
		void fly(int8_t x, int8_t y);

		// The part of a jumping mob's AI step that comes before its usual
		// decision. Returns whether it jumped instead.
		bool think_jump(const Physics &physics);

		// Jump on the next tick, holding direction dir. With check, turn
		// around if the jump lands at the height it started from.
		void jump(int8_t dir, bool check);

		// Which way the mob is pushing: -1 left, 0 not at all, 1 right.
		int8_t input() const;

		// Stand (0) or walk left (-1) or right (1).
		void walk(int8_t direction);

		// The weapon defense a blow meets.
		int32_t defense() const;

		// Whether a blow or Shadow Meso lands (physical_damage).
		bool blow_hits(const Attack &attack) const;

		// Roll one hit of fixed damage or Shadow Meso.
		std::pair<int32_t, bool> next_damage(const Attack &attack) const;

		// Roll a blow's hit `index`; Shadow Partner's copy the first set's.
		std::pair<int32_t, bool> next_blow(const Attack &attack, const std::vector<std::pair<int32_t, bool>> &hits,
										   size_t index) const;

		// Roll one hit of a spell (magic_damage).
		int32_t next_magic_damage(const Attack &attack) const;

		// Return the current 'head' position
		Point<int16_t> get_head_position(Point<int16_t> position) const;
		void draw_hp_bar(Point<int16_t> position) const;

		// Load the level tag in one of ShowLevel's sets: 0 white, 1 yellow,
		// 2 orange.
		void set_level_tag(uint8_t set);

		std::map<Stance, Animation> animations;

		// One of the mob's attacks (attackN/info, FUN_0067fd81). Ranges are
		// drawn for a mob facing left.
		struct AttackInfo
		{
			// 0 a box in front, 1 a direct shot, 2 a ball, 3 cells on the
			// ground, 4 cells at the mob's height.
			uint8_t type;
			bool magic;
			uint8_t element;
			int32_t watk;
			int32_t conmp;
			// Ms after the attack starts that it lands and that its effect
			// shows.
			int32_t after;
			int32_t effectafter;
			// Whether only a player standing on a foothold is hit.
			bool jump;
			bool tremble;
			bool deadly;
			bool mpburn;
			mob_attack::Box range;
			Point<int16_t> sp;
			int32_t r;
			mob_attack::Area area;
			int32_t attackcount;
			int32_t bulletspeed;
			// The mob's own action, and what shows on it, flies, lands on the
			// player and warns of a cell.
			Animation animation;
			std::optional<Animation> effect;
			std::optional<Animation> ball;
			Animation hit;
			std::optional<Animation> warning;
		};

		std::vector<AttackInfo> attacks;

		// One of the mob's skills (info/skill): it plays animation
		// skill<action> and shows the skill's effect `effectafter` ms in.
		struct SkillInfo
		{
			uint8_t skill;
			uint8_t level;
			uint8_t action;
			int32_t effectafter;
		};

		std::vector<SkillInfo> skills;
		// The skill<n> animations, by n.
		std::map<uint8_t, Animation> skillanimations;
		// The skill animation the mob shows, if any.
		std::optional<uint8_t> casting;
		// The skill the server last let it cast, and at what level.
		uint8_t pendingskill = 0;
		uint8_t pendingskilllevel = 0;
		// The attack the mob shows, if any, facing left if `attackleft`.
		std::optional<uint8_t> attacking;
		bool attackleft = false;
		// Ms more the AI stands still for its attack.
		int16_t attackhold = 0;
		int32_t mp;
		// Ms the mob has been shown, which its attacks are timed by, and
		// when it last made an area attack.
		int32_t clock = 0;
		int32_t lastarea = -10000;
		// An attack the controller made where its path starts, to play
		// next update.
		std::optional<std::pair<Pick, Point<int16_t>>> remoteattack;

		// A hit on its way: a box that hits the player where it is when
		// due, or a direct shot (no box) that hits them anyway.
		struct PendingHit
		{
			int32_t due;
			uint8_t index;
			bool left;
			std::optional<mob_attack::Box> box;
		};

		std::vector<PendingHit> pendinghits;
		// When an attack with `tremble` set lands, shaking the view.
		std::optional<int32_t> trembleat;

		// A ball flying from one point to another between two times.
		struct Ball
		{
			int32_t start;
			int32_t end;
			Point<int16_t> from;
			Point<int16_t> to;
			uint8_t index;
			bool left;
			Animation animation;
		};

		std::vector<Ball> balls;
		// An effect to show on the mob once due.
		struct PendingEffect
		{
			int32_t due;
			Animation animation;
			bool left;
			// Where the effect's `pos` puts it, if it has one.
			std::optional<int8_t> pos = std::nullopt;
		};

		std::vector<PendingEffect> pendingeffects;
		std::vector<Landed> landed;
		int32_t targetid = 0;
		mob_attack::Box targetbody{};
		bool targetonground = false;
		bool targetdarksight = false;
		// Its own look, while Doom shows another.
		std::optional<std::map<Stance, Animation>> ownlook;
		std::string name;
		Sound hitsound;
		Sound diesound;
		uint16_t level;
		float speed;
		float flyspeed;
		float chasespeed;
		// Whether the shoe was last set for a chase.
		bool chaseshoe = false;
		uint16_t watk;
		uint16_t matk;
		uint16_t wdef;
		uint16_t mdef;
		uint16_t accuracy;
		uint16_t avoid;
		uint16_t knockback;
		magic_damage::Attributes elements;
		bool undead;
		bool touchdamage;
		bool noflip;
		bool blocksteleport;
		bool notattack;
		bool firstattack;
		// It destroys itself (`selfDestruction/action` not 0): its last
		// attack is kept for that. One that bombs (action bit 2) tells the
		// server once the player is in reach, and has.
		bool selfdestructs;
		bool bombs;
		bool bombed;
		bool canmove;
		bool canjump;
		bool canfly;
		bool canchase;
		bool boss;
		// Its HP shows only in the boss HP bar, in these Gage colours (HPgaugeHide, hpTagColor,
		// hpTagBgcolor).
		bool gaugehide;
		uint8_t tagcolor;
		uint8_t tagbgcolor;
		// What Poison, Venom and Ambush show instead of their value.
		int32_t fixeddamage;
		// Its full HP, which caps what Power Guard sends back at it.
		int32_t maxhp;
		// Power Guard sends nothing back at it.
		bool invincible;
		// The only skills that hurt it (damagedBySelectedSkill), or none
		// for any attack.
		std::vector<int32_t> damagedby;
		// Only basic attacks hurt it (onlyNormalAttack).
		bool onlynormal;
		// It shows no name over its head (hideName), or no HP bar
		// (hideHP), when hit (0x668a64, 0x668a93).
		bool hidename;
		bool hidehp;
		// How wide the mob first stands, for how near Monster Magnet pulls
		// it (0x66486e).
		int16_t standwidth;

		// The statuses on the mob, by stat.
		std::map<uint8_t, MobStatParser::Status> statuses;
		MobHurts hurts;
		std::vector<int32_t> hurtnumbers;

		// The effects that last as long as their status, at their `pos`.
		struct StatusEffect
		{
			int32_t reason;
			uint8_t part;
			Animation animation;
			int8_t pos;
		};

		std::vector<StatusEffect> statuseffects;
		// The skill whose mark is on the mob, which no status takes off.
		int32_t marked = 0;

		EffectLayer effects;
		// Hit effects on the current frame's head, and those left at a point
		// in the field.
		EffectLayer headeffects;
		EffectLayer fieldeffects;
		Text namelabel;
		// "Lv. N", drawn left of the name tag (Basic.img/ShowLevel).
		std::vector<Texture> leveltag;
		// v83 skips the tag for bosses, mobs that hide their level, HP or
		// name, and 9xxxxxx mobs outside 94xxxxx and 96xxxxx.
		bool showleveltag;
		MobHpBar hpbar;
		Randomizer randomizer;

		TimedBool showhp;
		// With gaugehide, the boss HP bar comes down 20 s after the player last hit it, and keeps coming
		// down while it lives (0x66890a).
		bool hitbyplayer;
		TimedBool keepsgage;

		// Hits queued against this mob that haven't landed yet.
		uint16_t pending_hits;
		// Ticks before a killed mob dies even if its pending hits never land.
		uint16_t death_timeout;

		int32_t id;
		int8_t effect;
		int8_t team;
		bool dying;
		bool dead;
		// It leaves with an attack or skill, once that has played and hit;
		// blowing up waits for the next update to start.
		bool leaving;
		bool exploding;
		bool control = false;
		// v83's control state (CMob +0x130): 3 when the controller may send
		// a move, 4 when it may also attack or cast, 2 while a move awaits
		// the server's ack, 1 after a move sent without control, and below 0
		// when not controlling (FUN_006637ec).
		int8_t ctrlstate = -1;
		// The id of the last move sent, one more each move.
		int16_t moveid = 0;
		// What the mob shows (phobj, stance, flip) plays its paths, as v83's
		// displayed vecctrl (CMob +0x118) does. The controller runs the AI
		// on its own body ahead of that (+0x11c) and records the path it
		// sends next.
		move_path::Player shown;
		PhysicsObject body;
		Stance bodystance;
		bool bodyflip;
		move_path::Recorder recorder;
		// Whether the body starts over where the shown paths end.
		bool seated = false;
		// Whether a hit shows over the path until its animation ends.
		bool acting = false;
		// Ms more a hit mob holds still.
		int16_t hittime = 0;

		// A flinch for the AI to take next update: away from its attacker,
		// or toward the user Monster Magnet caught it for.
		struct Flinch
		{
			bool left;
			std::optional<Point<int16_t>> user;
		};

		std::optional<Flinch> flinch;
		// Whether the mob chases the local player, and for how many more ms.
		bool hastarget;
		int32_t targettime;
		Point<int16_t> targetpos;
		bool targetdead;
		// The Puppet it chases instead, if any, and where it stands.
		int32_t puppet = 0;
		Point<int16_t> puppetpos;
		// Ms until a firstAttack mob next reports the local player's distance.
		int16_t reporttime;
		// Ms towards the next AI step, and AI steps until the next decision.
		uint16_t aitime;
		int16_t thinktimer;
		// Where the AI means to go: -1 left, 0 nowhere, 1 right.
		int8_t movedir;
		// The direction held through a jump, and whether to check the
		// landing against jumpy, the height it jumped from.
		int8_t jumpdir;
		bool checkjump;
		int16_t jumpy;
		Stance stance;
		bool flip;
		// A flier heads for a waypoint, near its target or on a random platform
		// of the group it roams. flycount counts down the waypoints left
		// before it picks another group, or gives up the chase.
		bool haswaypoint;
		Point<int16_t> waypoint;
		int32_t flygroup;
		int32_t flycount;
		int8_t flyx;
		int8_t flyy;
		float walkforce;
		int8_t hppercent;
		bool fadein;
		Linear<float> opacity;

		// Monster Magnet's pull: toward where the user stood, how near it
		// stops, how fast it goes in px/s, and how far into its step it is.
		struct Pull
		{
			Point<int16_t> user;
			int16_t reach;
			double speed;
			int16_t elapsed;
		};

		std::optional<Pull> pull;
	};
}