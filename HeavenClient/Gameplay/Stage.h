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
#include "Combat/Dash.h"
#include "EmoteCooldown.h"
#include "QuestAlert.h"
#include "FameCooldown.h"
#include "SkillMacroRules.h"
#include "BlackoutRules.h"
#include "TrembleRules.h"
#include "MapleMap/MapInfo.h"
#include "MapleMap/MapTilesObjs.h"
#include "MapleMap/MapBackgrounds.h"
#include "MapleMap/MapPortals.h"
#include "MapleMap/MapChars.h"
#include "MapleMap/MapMobs.h"
#include "MapleMap/MapKites.h"
#include "MapleMap/MapMists.h"
#include "MapleMap/MapClouds.h"
#include "MapleMap/MapWeather.h"
#include "MapleMap/MapTooltips.h"
#include "MapleMap/MapDoors.h"
#include "MapleMap/MapSummons.h"
#include "MapleMap/MapReactors.h"
#include "MapleMap/MapNpcs.h"
#include "MapleMap/MapDrops.h"
#include "MapleMap/MapClock.h"
#include "MapleMap/MapShip.h"
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

		// Loads the map to display, with the player at a portal or, when the
		// server chases them there, at a point.
		void load(int32_t mapid, int8_t portalid, std::optional<Point<int16_t>> chase = std::nullopt);

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

		// Drop the rest of the skill macro running (v83 FUN_00631a80).
		void stop_macro();

		// The character under a cursor position, or 0 if there is none.
		int32_t character_at(Point<int16_t> cursorpos);
		// Someone else under a cursor position, by v83's box for opening the
		// menu on them, or 0. The player can't be picked.
		int32_t other_character_at(Point<int16_t> cursorpos);

		// The room whose balloon over someone else is under the cursor, or
		// 0; our own isn't (FUN_00972853).
		int32_t mini_room_at(Point<int16_t> cursorpos);

		// The kite under the cursor, newest first, or none.
		const MapKites::Kite *kite_at(Point<int16_t> cursorpos) const;

		// Lets go of a held key whose release went elsewhere, such as to a
		// dialog or a text field, so the player doesn't keep walking.
		void release_key(KeyType::Id keytype, int32_t keycode);

		// Lets go of every held key.
		void release_keys();

		// Lets go of a mouse press, wherever its release went.
		void release_cursor();

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

		// Returns a reference to the kites over the current map.
		MapKites &get_kites();

		// Returns a reference to the mists on the current map.
		MapMists &get_mists();

		// Returns a reference to the summons on the current map.
		MapSummons &get_summons();

		// Returns a reference to the Mystic Doors in the current field.
		MapDoors &get_doors();

		// Returns a reference to the drops on the current map.
		MapDrops &get_drops();

		// Returns a reference to the Player.
		Player &get_player();

		// The bulb over the player that offers auto quests.
		QuestAlert &get_quest_alert();

		// Return a reference to the attack and buff component.
		Combat &get_combat();

		// Whether no in-map move is waiting, so a dash may go.
		bool may_dash() const;
		// Whether a field is up, rather than being left or still fading in.
		bool is_active() const;
		// Move the player onto `spot` on the next update, as Assassinate's
		// second strike does (0x952d7b).
		void dash(teleport::Spot spot);

		// Returns the current map ID.
		int32_t get_mapid();

		// Returns the current map's info.
		const MapInfo &get_mapinfo() const;

		// Keeps the key the server gave the field, which map change and
		// portal script requests repeat.
		void set_field_key(uint8_t key);
		uint8_t get_field_key() const;

		// Return a pointer to a character, possibly the player.
		Optional<Char> get_character(int32_t cid);

		// FIELD_EFFECT 3: play Map.nx Effect.img/<path> once on the screen, above the UI when the
		// node has a z. False when there is no such effect.
		bool add_effect(std::string path);

		// FIELD_EFFECT 0: a summon's puff and sound at a point on the map.
		void add_summon_effect(uint8_t type, Point<int16_t> position);

		// A mob's attack `index` (attack1 is 0) hit `target`, the mob on the
		// side `left` gives (1 the right): its info/hit on them, at a random
		// spot on their body or following them when attached, and its
		// CharDam sound, fainter further from the player (FUN_009832e3,
		// FUN_009581a9).
		void show_mob_hit(Char &target, int32_t mobid, int8_t index, bool left);

		// Show a named map obj, such as a party quest's gate, in `state`, or in its current one again for -1.
		void set_object_state(const std::string &name, int32_t state);

		// Put every named trap back in its first state.
		void reset_obstacles();

		// SET_MAP_OBJECT_VISIBLE: show or hide the map objs with a tag.
		void show_tag(const std::string &tag, bool shown);

		// SET_BACK_EFFECT: fade one of the map's backs in (mode 0) or out (mode 1) over `duration` ms.
		void fade_back(uint8_t mode, uint8_t index, int32_t duration);

		// CLEAR_BACK_EFFECT: put the map's backs back as they were.
		void reset_backs();

		// Draw the screen effects that go above the UI.
		void draw_overlay(float alpha) const;

		// CONTI_MOVE: the map's ship comes or goes.
		void move_ship(uint8_t target, uint8_t event);

		// CONTI_STATE: where the map's ship is, which the client asks on
		// entering the map.
		void show_ship_state(uint8_t state, uint8_t flag);

		// Shake the field's view, not the UI, replacing any shake under way
		// (SetTremble 0x43da05). Unless forced, only with the ShakeScreen
		// option on.
		void tremble(double force, bool heavy, int32_t delay, int32_t additional, bool forced);

		// Lay black layers over the field, behind the player (FUN_004370cd).
		void black_out(int32_t layers);

		// Leave a Flash Jump's `effect` where a player jumped, flipped when
		// they face `right` (0x96c107, 0x97ffa4).
		void show_flash_jump(const char *effect, Point<int16_t> position, bool right);

		// A user effect's path ("Effect/BasicEff.img/Event1/Best", "UI/tutorial.img/25"): the NX file it names first,
		// then the node in it, also tried with the image's ".img" put back; null if there is none.
		static nl::node effect_node(const std::string &path);

		// User effect 14, a reward box's prize (0x9382b0): the prize's use sound, and the box's `path` played once
		// where the opener stands, not following them.
		void show_reward_effect(Point<int16_t> position, int32_t item_id, const std::string &path);

		// BLOW_WEATHER and a map's own weathers: start a 512xxxx item's weather with its message, or stop the one
		// blowing with item 0. One from the map or a whisper stops itself after 22 s.
		void blow_weather(int32_t item_id, const std::string &msg, bool from_map);

		// BLOW_WEATHER's flag, which keeps the message bar up through a stop.
		void set_weather_flag(uint8_t flag);

		// PLAY_JUKEBOX: a song plays until the player leaves the field (v83 CField+0x20).
		void start_jukebox();

		// Whether a weather item or a jukebox has to wait: a jukebox song plays, or the field's weather isn't over
		// (FUN_00a0ec80, FUN_00a0ec3b).
		bool effect_busy() const;

		// KILL_MOB: a monster left the field.
		void note_mob_left();

		// FIELD_EFFECT 6 changed the field's BGM, which a stopping weather plays again.
		void set_field_bgm(const std::string &path);

		// CLOCK kind 1: the time of day on the map's wall clock. A time sent
		// while the map loads waits for it.
		void set_clock(uint8_t hour, uint8_t minute, uint8_t second);

		// Whether the player may give fame now (once every 5 minutes); if
		// so, the wait starts.
		bool try_give_fame();

		// Pull one of the cash mist `item_id`'s `emotion` faces, picked by
		// their weights (ITEM_EMOTION, FUN_00938b3e), as an emote key would.
		void use_item_emotion(int32_t item_id);

		// Show the player's 501xxxx or 429xxxx effect item, or stop it if it is the one shown (FUN_00a24530).
		void use_item_effect(int32_t item_id);
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

		// Try the running macro's next skill; returns whether it went off
		// (v83 FUN_0063196b).
		bool try_macro_step(const skill_macro::Step& step);

		void respawn(int8_t portalid, std::optional<Point<int16_t>> chase);

		void check_portals();

		// Hurts the player on a trap and takes them where it sends them.
		void hit_obstacle();

		// Go through the Mystic Door the player stands in, if any
		// (FUN_005375ed in a town, FUN_007bd594 elsewhere).
		void check_doors(Point<int16_t> playerpos);

		// The Mystic Doors standing at the town's door portals.
		std::vector<MapDoors::Spot> get_town_doors() const;

		void check_touch_portals(Point<int16_t> drawn);

		void enter_portal(const Portal::WarpInfo &warpinfo);

		void use_jump_pad(const Portal &portal);

		// Returns whether the player was moved to the target.
		bool update_teleport();

		void use_teleport(int32_t skill);

		void use_flash_jump(int32_t skill);

		void use_wings(int32_t skill);

		void check_seats();

		void catch_ladders(Point<int16_t> before);

		void check_drops();

		// Each pet out with a Meso Magnet or an Item Pouch reaches for a
		// drop at its feet (FUN_00705c5d).
		void check_pet_loot();

		// Couples wearing their wedding rings near each other show the
		// ring's mark, and its big effect when close (CUserPool::Update
		// 0x9717b5).
		void show_rings();

		// Fear's darkness, all but the player's narrowed sight (FUN_0053868d).
		void draw_fear(double viewx, double viewy, float alpha) const;

		// Show the map objs whose tag is a quest's showLayerTag while that quest is in progress, and hide them
		// otherwise (FUN_00537cdc).
		void show_quest_tags();

		enum State
		{
			INACTIVE,
			TRANSITION,
			ACTIVE
		};

		Camera camera;
		// The view's shake, and when it next moves.
		tremble::State shake;
		int32_t next_shake = 0;
		// The black layers of Bamboo Rain and Rage of Pharaoh, and how
		// opaque they are now.
		blackout::State blackout;
		float blackout_alpha = 0.0f;
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
		// The ms since the quest-bound tags were last looked at, and the quest log they were looked at for.
		int32_t since_quest_tags = 0;
		uint64_t quest_tags_revision = 0;
		QuestAlert quest_alert;
		MapBackgrounds backgrounds;
		MapPortals portals;
		MapReactors reactors;
		MapNpcs npcs;
		MapChars chars;
		MapMobs mobs;
		MapDrops drops;
		MapMists mists;
		MapKites kites;
		// The ms since a mob's mist last hurt the player.
		int32_t since_mist_hurt = mist::HURT_INTERVAL + 1;
		// When a mob touching the barrel last tried to see through it.
		int64_t barrel_checked = INT32_MIN;
		MapDoors doors;
		MapSummons summons;
		MapShip ship;
		MapClouds clouds;
		MapWeather weather;
		bool jukebox = false;
		MapTooltips tooltips;
		MapClock clock;
		// A wall clock time that came before its map loaded.
		struct ClockTime
		{
			uint8_t hour;
			uint8_t minute;
			uint8_t second;
		};

		std::optional<ClockTime> pending_clock;
		// Effects left where they started, such as a jump pad's burst.
		EffectLayer ground_effects;
		// One-time FIELD_EFFECT effects: on the map, on the screen, and on the screen above the UI.
		EffectLayer field_effects;
		EffectLayer screen_effects;
		EffectLayer overlay_effects;
		Animation jump_effect;
		// What Fear leaves the player to see (Effect/MapEff.img/Viewrange/3).
		Texture fear_sight;

		Combat combat;
		skill_macro::Runner macro_runner;
		dash::Command dash_command;
		EmoteCooldown emote_cooldown;
		FameCooldown fame_cooldown;
		// When the last effect item was shown, in ms of uptime, or -1.
		int64_t item_effect_shown = -1;

		std::chrono::time_point<std::chrono::steady_clock> start;
		uint16_t levelBefore;
		int64_t expBefore;
	};
}