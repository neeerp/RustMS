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

#include <string>

#include <cstddef>

#include <cstdint>

#include "MapObject.h"

#include "../Movement.h"

#include "../Combat/Attack.h"
#include "../Combat/Bullet.h"

#include "../../Audio/Audio.h"
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
			int8_t team, Point<int16_t> position, const Physics &physics);

		// Draw the mob
		void draw(double viewx, double viewy, float alpha) const override;

		// Update movement and animations.
		int8_t update(const Physics &physics) override;

		// Change this mob's control mode:
		// 0 - no control, 1 - control, 2 - aggro
		void set_control(int8_t mode);

		// Tell the mob where the local player is, whom it chases with aggro.
		void set_target(Point<int16_t> position, bool dead);

		// Send movement to the mob.
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

		// Show an effect at the mob's position.
		void show_effect(const Animation &animation, int8_t pos, int8_t z, bool flip);

		// Show a weapon's hit spark where the attack meets the mob's body.
		void show_hit(const Animation &animation, const Rectangle<int16_t> &range, bool flip);

		// Calculate the damage to this mob with the specified attack.
		std::vector<std::pair<int32_t, bool>> calculate_damage(const Attack &attack);

		// Apply damage to the mob.
		void apply_damage(int32_t damage, bool toleft);

		// Create a touch damage attack to the player
		MobAttack create_touch_attack() const;

		// Check if touching this mob hurts: `bodyAttack` without `notAttack`.
		bool deals_touch_damage() const;

		// Check if this mob collides with the specified rectangle.
		bool is_in_range(const Rectangle<int16_t> &range) const;

		// Check if this mob is still alive.
		bool is_alive() const;

		// Return the head position.
		Point<int16_t> get_head_position() const;

	private:
		// Set the stance by byte value
		void set_stance(uint8_t stancebyte);

		// Set the stance by enum value.
		void set_stance(Stance newstance);

		// Start the death animation.
		void apply_death();

		// Take one 90 ms step of a ground mob's AI.
		void think(const Physics &physics);

		// Chase the local player for 180 to 240 s, deciding at once.
		void chase();

		// Set the walking and flying speeds, at chaseSpeed if chasing.
		void update_shoe(bool chasing);

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

		// Stand (0) or walk left (-1) or right (1), telling the server if
		// that changes anything.
		void walk(int8_t direction);

		// Send the current position and state to the server.
		void update_movement();

		// Calculate the hit chance
		float calculate_hitchance(int16_t leveldelta, int32_t accuracy) const;

		// Calculate the minimum damage.
		double calculate_mindamage(int16_t leveldelta, double mindamage, bool magic) const;

		// Calculate the maximum damage.
		double calculate_maxdamage(int16_t leveldelta, double maxdamage, bool magic) const;

		// Calculate a random damage line based on the specified values.
		std::pair<int32_t, bool> next_damage(double mindamage, double maxdamage, float hitchance, float critical) const;

		// Return the current 'head' position
		Point<int16_t> get_head_position(Point<int16_t> position) const;

		// Load the level tag in one of ShowLevel's sets: 0 white, 1 yellow,
		// 2 red.
		void set_level_tag(uint8_t set);

		std::map<Stance, Animation> animations;
		std::string name;
		Sound hitsound;
		Sound diesound;
		uint16_t level;
		float speed;
		float flyspeed;
		float chasespeed;
		uint16_t watk;
		uint16_t matk;
		uint16_t wdef;
		uint16_t mdef;
		uint16_t accuracy;
		uint16_t avoid;
		uint16_t knockback;
		bool undead;
		bool touchdamage;
		bool noflip;
		bool notattack;
		bool firstattack;
		bool canmove;
		bool canjump;
		bool canfly;
		bool canchase;

		EffectLayer effects;
		Text namelabel;
		// "Lv. N", drawn left of the name tag (Basic.img/ShowLevel).
		std::vector<Texture> leveltag;
		MobHpBar hpbar;
		Randomizer randomizer;

		TimedBool showhp;

		std::vector<Movement> movements;
		// Hits queued against this mob that haven't landed yet.
		uint16_t pending_hits;
		// Ticks before a killed mob dies even if its pending hits never land.
		uint16_t death_timeout;

		int32_t id;
		int8_t effect;
		int8_t team;
		bool dying;
		bool dead;
		bool control;
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
		// AI steps since the flier last told the server where it is.
		uint16_t flysteps;
		float walkforce;
		int8_t hppercent;
		bool fading;
		bool fadein;
		Linear<float> opacity;
	};
}