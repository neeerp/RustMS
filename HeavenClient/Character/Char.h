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

#include <array>
#include <map>
#include <string>
#include <vector>

#include <cstddef>

#include <cstdint>

#include "ChalkboardBalloon.h"
#include "CharEffect.h"
#include "GuildMark.h"
#include "ItemEffect.h"
#include "MedalTag.h"
#include "MorphRules.h"
#include "MiniRoomBalloon.h"

#include "Inventory/Weapon.h"
#include "Look/Afterimage.h"
#include "Look/CharLook.h"
#include "Look/PetLook.h"

#include "../Audio/Audio.h"
#include "../Template/Optional.h"

#include "../Graphics/EffectLayer.h"

#include "../Gameplay/Combat/ComboOrbs.h"
#include "../Gameplay/Combat/DamageNumber.h"
#include "../Gameplay/MapleMap/MapObject.h"
#include "../IO/Components/ChatBalloon.h"

#include "math.h"

namespace ms
{
	// Base for characters, e.g. the player and other clients on the same map.
	class Char : public MapObject
	{
	public:
		// Player states which determine animation and state 
		// Values are used in movement packets (Add one if facing left)
		enum State : int8_t
		{
			WALK = 2,
			STAND = 4,
			FALL = 6,
			ALERT = 8,
			PRONE = 10,
			SWIM = 12,
			LADDER = 14,
			ROPE = 16,
			DIED = 18,
			SIT = 20
		};

		static State by_value(int8_t value)
		{
			return static_cast<State>(value);
		}

		// Draw look, NameTag, effects and chat bubble.
		void draw(double viewx, double viewy, float alpha) const override;
		// Draw look
		void draw_preview(Point<int16_t> position, float alpha) const;
		// Update look and movements
		int8_t update(const Physics& physics) override;
		// Return the current map layer, or seven if on a ladder or rope.
		int8_t get_layer() const override;

		// Check whether the character is invincible
		virtual bool is_invincible() const;

		// Return the character's level.
		virtual uint16_t get_level() const = 0;

		// Return the character's level.
		virtual int32_t get_skilllevel(int32_t skillid) const = 0;

		// Return the charge skill (Weapon Charge) on the character's
		// weapon, or 0.
		virtual int32_t get_charge() const;

		// Return the character's base attacking speed.
		virtual int8_t get_integer_attackspeed() const = 0;

		// Return the speed stat that paces the walk animation. Others walk
		// at the base 100.
		virtual int32_t get_walk_animation_speed() const;

		// Return the attack speed as a multiplier
		float get_real_attackspeed() const;

		// Return the attack speed degree that stretches an action's frames,
		// from 2 (fastest) to 10 (slowest); 6 plays them as drawn.
		int8_t get_action_speed() const;

		// Return the delay until applying an attack.
		uint16_t get_attackdelay(size_t no) const;

		// How long the attack under way plays at the character's speed.
		uint16_t get_attack_length() const;

		// How long a body action plays at the character's speed.
		int32_t get_action_duration(const std::string &action) const;

		// How long a skill's body action keeps the character from acting.
		uint16_t get_action_lock(const std::string &action) const;

		// Set if the character sprite is mirrored (true = facing right)
		virtual void set_direction(bool flipped);

		// Change the character's state.
		virtual void set_state(State newstate);

		// Change the character's stance to an attack action.
		void attack(const std::string &action);

		// Change the character's stance to an attack stance.
		void attack(Stance::Id stance);

		// Change the character's stance to it's regular attack.
		void attack(bool degenerate);

		// Change the character's stance to a regular attack stance picked
		// with CharLook::get_attackstance.
		void attack(bool degenerate, Stance::Id stance);

		// Hold the attack on this frame until end_attack.
		void hold_attack(uint8_t frame);

		// Play the action just started at this speed rather than the
		// character's own, until the next one. v83 plays the action of a
		// skill used without attacking at 6 (0x96d4a6).
		void fix_action_speed(int8_t speed);

		// End the attack now, held or not.
		void end_attack();

		// Set the afterimage for an attack.
		void set_afterimage(int32_t skill_id);

		// Return the current afterimage.
		const Afterimage &get_afterimage() const;

		// Return the range of a regular attack's afterimage in a stance.
		Rectangle<int16_t> get_afterimage_range(Stance::Id stance) const;

		// Display an animation as an effect with the character
		void show_attack_effect(Animation animation, int8_t z);

		// Display a held skill's effect with the character, once or over and
		// over until clear_held_effects.
		void show_held_effect(Animation animation, int8_t z, bool looping);

		void clear_held_effects();

		// Show the `affected` animation of each mob skill (by its reason,
		// skill | level << 16) that gave the character a disease, for as
		// long as it lasts (FUN_0093e344 keeps the list, FUN_00932da6
		// shows the new ones).
		void show_afflictions(const std::vector<int32_t> &reasons);

		// Display an animation as an effect on top of the character.
		void show_effect_id(CharEffect::Id toshow);

		// Show Combo Attack's orbs: the buff's value less one, or -1
		// without it. A character coming into view (`init`) shows them
		// without animating (FUN_0093cdbc).
		void show_combo(int32_t orbs, bool init);

		// A finisher's burst from the character's feet, by the orbs it
		// spends, never mirrored (0x934725).
		void show_combo_finish(int32_t skill);

		// Energy Charge's glow while the gauge is full (0x93e2e5, FUN_00930b27): the skill's `effect` once with
		// its `Use` sound, unless the character just came into view, then the same over and over from 1.35 s on.
		void show_energy(int32_t skill, bool full, bool init);
		// Loop Berserk's aura, or take it away (SHOW_ITEM_GAIN_INCHAT 1).
		void show_berserk(bool on);

		// Dash's dust, the skill's `special` looping at the feet while the character dashes on a foothold and
		// hidden in the air (FUN_00930b27); a skill of 0 takes it away.
		void show_dash(int32_t skill);

		// Wings' glide: its `special` looping at the feet with its `Loop` sound while the character glides, and its
		// `finish` once on landing from one (FUN_00930b27).
		void glide(bool gliding, bool landed);

		// Play an animation anchored to the character (e.g. a tutorial image).
		void show_effect(const Animation &effect);

		// Display the iron body skill animation.
		void show_iron_body();

		// Display a skill or mastery book's glow: its first layer over the
		// character, its second behind (v83 FUN_0043cba9, z 3 and -1).
		void show_skill_book(bool success);

		// Display damage over the characters head, with the hit face for
		// 1.5 s if it hurt, as v83 does for every user (0x9832e3).
		void show_damage(int32_t damage);

		// Display a heal over the characters head in blue.
		void show_heal(int32_t amount);

		// Display a chat bubble with the specified line in it.
		void speak(const std::string &line);

		// Show an instruction balloon above the character (PLAYER_HINT).
		void show_hint(const std::string &text, int16_t width);

		// Change a part of the character's look.
		void change_look(MapleStat::Id stat, int32_t id);

		// Change the character's state by id.
		void set_state(uint8_t statebyte);

		// Change the character's face expression by id.
		void set_expression(int32_t expression);

		// Show a face for this many ms.
		void set_expression(Expression::Id expression, int32_t hold);

		// SPAWN_PET (168): a pet joins the list at `index`, moving those
		// from there up, or replaces the one there. `local` is the
		// player's own.
		void add_pet(uint8_t index, bool replace, const PetEntry &pet, bool local);

		// The pet at `index` leaves the list, and those after it move down.
		void remove_pet(uint8_t index);

		// Forget the pets, as on a new map, where they are sent again.
		void clear_pets();

		// The serials of the pets in list order, 0 for none.
		std::array<int64_t, 3> get_pet_serials() const;

		// MOVE_PET (170): where the pet at `index` went.
		void send_pet_movement(uint8_t index, const std::vector<Movement> &movements);

		// The pet at `index`, if there is one.
		Optional<PetLook> get_pet(uint8_t index);

		// Return if the character is facing left
		bool getflip() const;
		// Where the face is drawn from the feet: the look's face position, mirrored when facing right.
		Point<int16_t> get_face_offset() const;
		// Return the name of this character
		std::string get_name() const;
		// The guild tag under the character's name: the guild's name, or ""
		// for no tag, and its emblem.
		void set_guild_name(const std::string &name);
		void set_guild_emblem(const GuildEmblem &emblem);

		// Shows the balloon of the room the character keeps open, or takes
		// it down.
		void set_mini_room(const std::optional<MiniRoomBox> &box);

		// The room whose balloon is at `cursor`, or 0, with the feet at `feet`
		// on the screen.
		int32_t mini_room_at(Point<int16_t> feet, Point<int16_t> cursor) const;

		// Puts up the character's chalkboard with `text`, or takes it down.
		// Only the player's own board can be closed.
		void set_chalkboard(const std::optional<std::string> &text, bool closable);
		// The note on the character's chalkboard, if one is up.
		std::optional<std::string> get_chalkboard() const;
		// Tracks the chalkboard's close button, with the feet at `feet` on the
		// screen. True when it was clicked.
		bool send_chalkboard_cursor(Point<int16_t> feet, Point<int16_t> cursor, bool pressed, bool held);
		void release_chalkboard_cursor();
		bool on_chalkboard_close(Point<int16_t> feet, Point<int16_t> cursor) const;

		// Return if the char is in the Char::State::SIT state
		bool is_sitting() const;

		// Return if the char is in the Char::State::PRONE state
		bool is_prone() const;

		// Return whether the char is lying dead under a tombstone.
		bool is_dead() const;

		// Sit on the portable chair itemid, or get up from it with 0.
		virtual void set_chair(int32_t itemid);

		// Return the portable chair the character sits on, or 0.
		int32_t get_chair() const;

		// Show an effect item's effect (501xxxx or 429xxxx), or stop it with 0.
		void set_item_effect(int32_t item_id);

		// The effect item shown, or 0.
		int32_t get_item_effect() const;

		// Start the effect over, as on a new field.
		void restart_item_effect();

		// Return if the char is in the Char::LADDER or Char::ROPE state.
		bool is_climbing() const;

		// Get on a mount (190xxxx or 193xxxx), or off with 0. The look follows a moment later, while the Monster
		// Rider effect plays (FUN_00936859); a rider coming into view rides `at_once`.
		void ride(int32_t vehicle, bool at_once);

		// The mount ridden, 0 for none.
		int32_t get_vehicle() const;

		// Whether the look is still to get on or off (+0x1f4c).
		bool is_switching_vehicle() const;

		// How riding moves, or nothing when not riding a mount with a template.
		const std::optional<TamingMob::Ability> &get_ride_ability() const;

		// The tamed monster's level, EXP and fatigue (SET_TAMING_MOB_INFO). A tired one stands tired.
		void set_mount_info(int32_t level, int32_t exp, int32_t fatigue);

		// The tamed monster reached a new level.
		void show_mount_levelup();

		// Look like a Morph.nx template, or as before with 0, in a puff of smoke unless coming into view
		// `at_once` (FUN_00941d4b).
		void transform(int32_t morph, bool at_once);

		// The item's morph or barrel the character looks like, if any: one that moves by its template
		// (FUN_004520a4, FUN_004520d5).
		const MorphTemplate *get_template_morph() const;

		// Whether the character is hidden in Oak Barrel.
		bool is_barrelled() const;

		// Whether it is Transformation's superman.
		bool is_superman() const;

		// Whether Oak Barrel is dropping onto the character or breaking.
		bool is_barrel_busy() const;

		// Play a mount skill's effect of getting on or off.
		void show_ride_effect(int32_t skill);

		// Return whether the character sprite uses stances for two-handed weapons.
		bool is_twohanded() const;

		// Return the type of the currently equipped weapon.
		Weapon::Type get_weapontype() const;

		// Obtain a reference to this character's look.
		CharLook &get_look();

		// Obtain a const reference to this character's look.
		const CharLook &get_look() const;

		// Return a reference to this characters's physics.
		PhysicsObject &get_phobj();
		const PhysicsObject &get_phobj() const;

		// Initialize character effects
		static void init();

	protected:
		Char(int32_t oid, const CharLook &look, const std::string &name);

		// Update the character's animation with the given speed.
		bool update(const Physics &physics, float speed);

		// Get a speed modifier for the current stance.
		float get_stancespeed() const;

		// CUserLocal is invincible for 1500 ms after a hit, and blinks
		// only if the hit did damage (FUN_009581a9). Others never are.
		void become_invincible(bool blink);

		CharLook look;
		CharLook look_preview;
		std::array<PetLook, 3> pets;

		State state = State::STAND;
		bool attacking;
		// The speed fixed for the current action, or 0 for the character's own.
		int8_t fixed_action_speed = 0;
		// The milliseconds of animation time a speed left over from its last
		// whole step.
		float stance_carry = 0.0f;
		bool facing_right;
		int32_t chair = 0;
		int32_t vehicle = 0;
		int16_t vehicle_switch = 0;
		std::optional<TamingMob::Ability> ride_ability;
		int32_t mount_level = 0;
		int32_t mount_exp = 0;
		int32_t mount_fatigue = 0;

	private:
		Text namelabel;
		Text guildlabel;
		GuildMark guildmark;
		MedalTag medaltag;
		ChalkboardBalloon chalkboard;
		ChatBalloon chatballoon;
		ChatBalloon hintballoon;
		MiniRoomBalloon miniroom;
		EffectLayer effects;
		EffectLayer held_effects;
		EffectLayer energy_glow;
		EffectLayer berserk_aura;
		// The full gauge's effect, and how long until it loops; -1 once it does.
		std::vector<Animation> energy_effect;
		int16_t energy_wait = -1;
		morph::Barrel barrel;
		Animation dash_dust;
		bool dashing = false;
		Animation wings_glow;
		Sound wings_sound;
		bool gliding = false;
		Afterimage afterimage;
		// A chair's effect and effect2 (v83 0x93c7c3), each on the face when
		// its pos is 1 and on the feet otherwise; a negative z draws it
		// behind the character.
		struct ChairLayer
		{
			Animation animation;
			bool at_face;
			bool in_front;
		};

		std::vector<ChairLayer> chair_layers;

		ItemEffect item_effect;

		// A disease's animation, on the face when its pos is 1 like the
		// chair's.
		struct Affliction
		{
			Animation animation;
			bool at_face;
		};

		std::map<int32_t, Affliction> afflictions;

		combo::Orbs combo_orbs;

		TimedBool invincible;
		bool blinking = false;
		TimedBool ironbody;
		std::list<DamageNumber> damagenumbers;

		// As v83 does it (FUN_0092fd5a): a dead character's tombstone drops 600 pixels onto it in 880 ms
		// while the ghost fades in over 2.5 seconds. The ghost circles a point 20 pixels above the
		// tombstone: clockwise from the top, 10 pixels out, once every two seconds.
		static constexpr int16_t TOMB_DROP = 600;
		static constexpr uint16_t TOMB_FALL = 880 / Constants::TIMESTEP;
		static constexpr float GHOST_FADE = Constants::TIMESTEP / 2500.0f;
		static constexpr float GHOST_RADIUS = 10.0f;
		static constexpr int16_t GHOST_RISE = 20;
		static constexpr uint16_t GHOST_LAP = 2000 / Constants::TIMESTEP;
		Animation tomb;
		uint16_t tomb_tick = 0;
		bool tomb_landed = false;
		Linear<float> ghost_opacity;
		uint16_t ghost_tick = 0;

		static EnumMap<CharEffect::Id, Animation> chareffects;
		static Animation tomb_fall;
		static Animation tomb_land;
		// BasicEff.img/SkillBook: [failure, success][front, back]
		static Animation skill_book[2][2];
		// Combo Attack's and Advanced Combo's state/0 to 5: the center,
		// then the orbs.
		static Texture combo_sprites[2][6];

		void draw_combo(Point<int16_t> position) const;
	};
}