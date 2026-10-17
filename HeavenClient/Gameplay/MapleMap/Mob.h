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
#include "MobHurts.h"

#include "../Movement.h"
#include "../MovePath.h"

#include "../Combat/Attack.h"
#include "../Combat/Bullet.h"
#include "../Combat/MagicDamage.h"

#include "../../Audio/Audio.h"
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

		// Tell the mob where the local player is, whom it chases with aggro.
		void set_target(Point<int16_t> position, bool dead);

		// The server got the move with this id (FUN_0066c23b), saying whether
		// the mob may attack or cast next.
		void acknowledge(int16_t moveid, bool nextattack);

		// Queue the controller's path to play (0xEF).
		void send_movement(Point<int16_t> start, std::vector<Movement> &&movements);

		// Kill the mob with the appropriate type:
		// 0 - make inactive 1 - death animation 2 - fade out
		// The death animation waits for any hits still on their way.
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

		// Step the AI ahead until it has a path to send, then send it and
		// play it (FUN_0066b6fc).
		void plan(const Physics &physics);

		// One step of the AI and its physics, recorded into the path.
		void step_body(const Physics &physics);

		// Stop what is shown at the next foothold, flinch from there, and
		// send the path at once.
		void flinch_body(const Physics &physics);

		// Start the death animation.
		void apply_death();

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

		// Calculate the hit chance
		float calculate_hitchance(int16_t leveldelta, int32_t accuracy) const;

		// Calculate the minimum damage.
		double calculate_mindamage(int16_t leveldelta, double mindamage) const;

		// Calculate the maximum damage.
		double calculate_maxdamage(int16_t leveldelta, double maxdamage) const;

		// Calculate a random damage line based on the specified values.
		std::pair<int32_t, bool> next_damage(double mindamage, double maxdamage, float hitchance, float critical,
											 const Attack &attack) const;

		// Roll one hit of a spell (magic_damage).
		int32_t next_magic_damage(const Attack &attack) const;

		// Return the current 'head' position
		Point<int16_t> get_head_position(Point<int16_t> position) const;
		void draw_hp_bar(Point<int16_t> position) const;

		// Load the level tag in one of ShowLevel's sets: 0 white, 1 yellow,
		// 2 orange.
		void set_level_tag(uint8_t set);

		std::map<Stance, Animation> animations;
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
		bool canmove;
		bool canjump;
		bool canfly;
		bool canchase;
		bool boss;
		// What Poison, Venom and Ambush show instead of their value.
		int32_t fixeddamage;
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

		// Hits queued against this mob that haven't landed yet.
		uint16_t pending_hits;
		// Ticks before a killed mob dies even if its pending hits never land.
		uint16_t death_timeout;

		int32_t id;
		int8_t effect;
		int8_t team;
		bool dying;
		bool dead;
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
		bool fading;
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