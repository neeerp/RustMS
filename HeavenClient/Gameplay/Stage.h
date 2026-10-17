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

#include <cstdint>
#include <optional>

#include "Combat/Combat.h"
#include "EmoteCooldown.h"
#include "FameCooldown.h"
#include "MapleMap/MapInfo.h"
#include "MapleMap/MapTilesObjs.h"
#include "MapleMap/MapBackgrounds.h"
#include "MapleMap/MapPortals.h"
#include "MapleMap/MapChars.h"
#include "MapleMap/MapMobs.h"
#include "MapleMap/MapMists.h"
#include "MapleMap/MapSummons.h"
#include "MapleMap/MapReactors.h"
#include "MapleMap/MapNpcs.h"
#include "MapleMap/MapDrops.h"
#include "MapleMap/MapEffect.h"
#include "Physics/Physics.h"

#include "../Character/Player.h"
#include "../Graphics/EffectLayer.h"
#include "../IO/KeyType.h"

namespace ms
{
	class Stage : public Singleton<Stage>
	{
	public:
		Stage();

		void init();

		// Loads the map to display
		void load(int32_t mapid, int8_t portalid);

		// Remove all map objects and graphics.
		void clear();

		// Construct the player from a character entry.
		void loadplayer(const CharEntry &entry);

		// Call 'draw()' of all objects on stage
		void draw(float alpha) const;

		// Calls 'update()' of all objects on stage.
		void update();

		// Show a character effect
		void show_character_effect(int32_t cid, CharEffect::Id effect);

		// Send key input to the stage
		void send_key(KeyType::Id keytype, int32_t keycode, bool pressed);

		// Talk to the nearest NPC in reach, as if it had been clicked.
		void talk_to_nearest_npc();

		// The character under a cursor position, or 0 if there is none.
		int32_t character_at(Point<int16_t> cursorpos);

		// Lets go of a held key whose release went elsewhere, such as to a
		// dialog or a text field, so the player doesn't keep walking.
		void release_key(KeyType::Id keytype, int32_t keycode);

		// Lets go of every held key.
		void release_keys();

		// Send mouse input to the stage.
		Cursor::State send_cursor(bool pressed, Point<int16_t> position);

		// Check if the specified id is the player's id
		bool is_player(int32_t cid) const;

		// Returns a reference to the NPCs on the current map.
		MapNpcs &get_npcs();

		// Returns a reference to the other characters on the current map.
		MapChars &get_chars();

		// Returns a reference to the mobs on the current map.
		MapMobs &get_mobs();

		// Returns a reference to the reactors on the current map.
		MapReactors &get_reactors();

		// Returns a reference to the mists on the current map.
		MapMists &get_mists();

		// Returns a reference to the summons on the current map.
		MapSummons &get_summons();

		// Returns a reference to the drops on the current map.
		MapDrops &get_drops();

		// Returns a reference to the Player.
		Player &get_player();

		// Return a reference to the attack and buff component.
		Combat &get_combat();

		// Returns the current map ID.
		int32_t get_mapid();

		// Keeps the key the server gave the field, which map change and
		// portal script requests repeat.
		void set_field_key(uint8_t key);
		uint8_t get_field_key() const;

		// Return a pointer to a character, possibly the player.
		Optional<Char> get_character(int32_t cid);

		// Set a map effect
		void add_effect(std::string path);

		// Whether the player may give fame now (once every 5 minutes); if
		// so, the wait starts.
		bool try_give_fame();
		// Get the total up time of the client
		int64_t get_uptime();
		// Get the level at the start of the client
		uint16_t get_uplevel();
		// Get the EXP at the start of the client
		int64_t get_upexp();

		// Notify the server that the player has moved maps
		void transfer_player();

		// Ask to move to another channel, unless the player is alert (and
		// not a GM) or the map forbids it, which a notice explains. Return
		// whether the request went.
		bool change_channel(uint8_t channel);

		// Ask to enter the Cash Shop, with the same refusals; GMs too must
		// catch their breath.
		void enter_cash_shop();

	private:
		void load_map(int32_t mapid);

		void respawn(int8_t portalid);

		void check_portals();

		void check_touch_portals(Point<int16_t> drawn);

		void enter_portal(const Portal::WarpInfo &warpinfo);

		void use_jump_pad(const Portal &portal);

		// Returns whether the player was moved to the target.
		bool update_teleport();

		void use_teleport(int32_t skill);

		void check_seats();

		void catch_ladders(Point<int16_t> before);

		void check_drops();

		enum State
		{
			INACTIVE,
			TRANSITION,
			ACTIVE
		};

		Camera camera;
		Physics physics;
		Player player;

		Optional<Playable> playable;
		State state;
		int32_t mapid;
		uint8_t field_key = 0;

		MapInfo mapinfo;
		// An in-map move waiting for the next update: by portal, or by skill
		// onto a foothold.
		struct Teleport
		{
			Point<int16_t> target;
			std::optional<uint16_t> foothold;
		};

		std::optional<Teleport> pending_teleport;
		// The ms until the player may move in the map again.
		int16_t teleport_cooldown = 0;
		MapTilesObjs tilesobjs;
		MapBackgrounds backgrounds;
		MapPortals portals;
		MapReactors reactors;
		MapNpcs npcs;
		MapChars chars;
		MapMobs mobs;
		MapDrops drops;
		MapMists mists;
		MapSummons summons;
		MapEffect effect;
		// Effects left where they started, such as a jump pad's burst.
		EffectLayer ground_effects;
		Animation jump_effect;

		Combat combat;
		EmoteCooldown emote_cooldown;
		FameCooldown fame_cooldown;

		std::chrono::time_point<std::chrono::steady_clock> start;
		uint16_t levelBefore;
		int64_t expBefore;
	};
}