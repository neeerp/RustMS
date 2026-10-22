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

#include "MapObjects.h"
#include "Mob.h"

#include "../Spawn.h"

#include "../Combat/Attack.h"
#include "../Combat/MesoExplosion.h"
#include "../Combat/SpecialMove.h"

#include <queue>

namespace ms
{
	// A collection of mobs on a map.
	class MapMobs
	{
	public:
		// Draw all mobs on a layer.
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// Update all mobs. Those with aggro chase the local player, whom all
		// their attacks may hit. The `reporter` tells the server of mobs
		// that run into a friendly mob.
		void update(const Physics &physics, const Mob::Target &target, bool reporter);

		// The local player's hits from mob attacks since last asked.
		std::vector<Mob::Landed> take_attacks();

		// Spawn a new mob.
		void spawn(MobSpawn &&spawn);

		// Kill a mob.
		void remove(int32_t oid, int8_t effect);

		// Remove all mobs.
		void clear();

		// Update who a mob is controlled by.
		void set_control(int32_t oid, bool control);

		// Update a mob's hp display.
		void send_mobhp(int32_t oid, int8_t percent, uint16_t playerlevel);

		// Show a number on a mob: damage, a heal if negative, or a miss.
		void show_damage(int32_t oid, int32_t damage);

		// Whether a mob is a friendly, which mobs hurt rather than players.
		bool is_friendly(int32_t oid) const;

		// The server's word of a hit on a friendly mob, with its HP after.
		void friendly_hit(int32_t oid, int32_t damage, int32_t hp, int32_t maxhp);

		// Show on a mob what another player's Power Guard or Mana Reflection
		// sent back at it.
		void show_reflect(int32_t oid, uint8_t percent, int32_t damage, bool physical);

		// What a mob's counter hurts the player with for one of their
		// attacks, or 0.
		int32_t counter(int32_t oid, bool magic) const;

		// Put temporary statuses on a mob, showing their effects.
		void set_statuses(int32_t oid, const std::vector<MobStatParser::Status> &statuses);

		// Take temporary statuses off a mob.
		void reset_statuses(int32_t oid, const BuffParser::Flags &flags);

		// The server got a mob's move.
		void acknowledge(int32_t oid, int16_t moveid, bool nextattack, int16_t mp, uint8_t skill, uint8_t skilllevel);

		// Update a mob's movements, and what it did where they start.
		void send_movement(int32_t oid, Point<int16_t> start, std::vector<Movement> &&movements, int8_t action,
						   bool left, int32_t skilldata);

		// Calculate the results of an attack.
		void
		send_attack(AttackResult &result, const Attack &attack, const std::vector<int32_t> &targets, uint8_t mobcount);

		// Note hits that will land on a mob once their attack connects;
		// a negative count cancels hits that won't land after all.
		void expect_hits(int32_t oid, int32_t count);

		// Applies damage to a mob.
		void apply_damage(int32_t oid, int32_t damage, bool toleft, const AttackUser &user, const SpecialMove &move);

		// A summon's hit lands on a mob, with the summon's spark.
		void apply_summon_damage(int32_t oid, int32_t damage, bool toleft, const Animation &hit);

		// Monster Magnet caught a mob for the user at `user`.
		void catch_mob(int32_t oid, Point<int16_t> user);

		// Check if the mob with the specified oid exists.
		bool contains(int32_t oid) const;

		// Return the id of the first mob who collides with the object, which
		// may be `hidden` by Dark Sight.
		int32_t find_colliding(const MovingObject &moveobj, int32_t fieldlimit, bool hidden) const;

		// The first mob whose touch hurts that overlaps `area`, or 0.
		int32_t find_touching(const Rectangle<int16_t> &area, int32_t fieldlimit = 0, bool hidden = false) const;

		// The first mob of template `mobid` to have entered the map that is
		// alive and overlaps `area`, which a catch item may be thrown at, or 0.
		int32_t find_catchable(const Rectangle<int16_t> &area, int32_t mobid) const;

		// The player's Puppet came, or went: the mobs this client moves turn
		// on it, or give it up.
		void aggro_puppet(int32_t oid, Point<int16_t> position);
		void release_puppet(int32_t oid);

		// Roll what mob `oid`'s touch does to a Puppet of skill level `level`.
		int32_t roll_puppet_damage(int32_t oid, int32_t level) const;

		// Create an attack by the specified mob.
		MobAttack create_attack(int32_t oid) const;

		// Return the position of a mob.
		Point<int16_t> get_mob_position(int32_t oid) const;

		// Return the head position of a mob.
		Point<int16_t> get_mob_head_position(int32_t oid) const;

		// A mob's template id, or 0 if there is no such mob.
		int32_t get_mob_id(int32_t oid) const;

		// The numbers Poison, Venom and Ambush showed on each mob since last
		// asked, with where its head is.
		std::vector<std::pair<Point<int16_t>, int32_t>> take_hurts();

		// Return a mob's body bounds on the map.
		Rectangle<int16_t> get_mob_body(int32_t oid) const;

		// The mobs Meso Explosion may hurt: those alive, in the order they
		// entered the map.
		std::vector<meso_explosion::Mob> explosion_targets() const;

		// Meso Explosion's lines on a mob from drops of these `mesos`.
		std::vector<int32_t> explode(int32_t oid, int32_t x, const std::vector<int32_t> &mesos) const;

		// What a number of Meso Explosion's shows on a mob.
		int32_t shown_explosion(int32_t oid, int32_t number) const;

		// Return all mob map objects
		MapObjects *get_mobs();

	private:
		// The friendly mob's pull on the mobs that hurt it, and the reports
		// of those that run into it (FUN_006675a8, 0x667e40).
		void update_friendly(int32_t charid, bool reporter);

		MapObjects mobs;

		std::queue<MobSpawn> spawns;

		// The ms the mobs have run, the friendly on the map, if any (one at
		// a time, the last to come), and when this client last reported a
		// hit on it, which its coming counts as (0x679565).
		int64_t clock = 0;
		int32_t friendlyoid = 0;
		int64_t reportedat = 0;
	};
}