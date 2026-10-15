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

#include "CharEffect.h"

#include "Inventory/Weapon.h"
#include "Look/Afterimage.h"
#include "Look/CharLook.h"
#include "Look/PetLook.h"

#include "../Graphics/EffectLayer.h"

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

		// Return the character's base attacking speed.
		virtual int8_t get_integer_attackspeed() const = 0;

		// Return the attack speed as a multiplier
		float get_real_attackspeed() const;

		// Return the delay until applying an attack.
		uint16_t get_attackdelay(size_t no) const;

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

		// Set the afterimage for an attack.
		void set_afterimage(int32_t skill_id);

		// Return the current afterimage.
		const Afterimage &get_afterimage() const;

		// Display an animation as an effect with the character
		void show_attack_effect(Animation animation, int8_t z);

		// Display an animation as an effect on top of the character.
		void show_effect_id(CharEffect::Id toshow);

		// Play an animation anchored to the character (e.g. a tutorial image).
		void show_effect(const Animation &effect);

		// Display the iron body skill animation.
		void show_iron_body();

		// Display damage over the characters head.
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

		// Add a pet with the specified stats.
		void add_pet(uint8_t index, int32_t iid, const std::string &name, int32_t uniqueid, Point<int16_t> pos,
					 uint8_t stance, int32_t fhid);

		// Remove a pet with the specified index and reason.
		void remove_pet(uint8_t index, bool hunger);

		// Return if the character is facing left
		bool getflip() const;
		// Return the name of this character
		std::string get_name() const;

		// Return if the char is in the Char::State::SIT state
		bool is_sitting() const;

		// Sit on the portable chair itemid, or get up from it with 0.
		virtual void set_chair(int32_t itemid);

		// Return the portable chair the character sits on, or 0.
		int32_t get_chair() const;

		// Return if the char is in the Char::LADDER or Char::ROPE state.
		bool is_climbing() const;

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

		// Initialize character effects
		static void init();

	protected:
		Char(int32_t oid, const CharLook &look, const std::string &name);

		// Update the character's animation with the given speed.
		bool update(const Physics &physics, float speed);

		// Get a speed modifier for the current stance.
		float get_stancespeed() const;

		CharLook look;
		CharLook look_preview;
		PetLook pets[3];

		State state = State::STAND;
		bool attacking;
		bool facing_right;
		int32_t chair = 0;

	private:
		Text namelabel;
		ChatBalloon chatballoon;
		ChatBalloon hintballoon;
		EffectLayer effects;
		Afterimage afterimage;
		// The chair's effect sits on the face when its pos is 1 and on the
		// feet otherwise; a negative z draws it behind the character.
		Animation chair_effect;
		bool chair_at_face = false;
		bool chair_in_front = false;
		TimedBool invincible;
		TimedBool ironbody;
		std::list<DamageNumber> damagenumbers;

		// A dead character's ghost fades in over two seconds while its tombstone drops from above behind it.
		// The ghost then circles the tombstone, as in the original client: clockwise, 9 pixels out, once
		// every two seconds.
		static constexpr int16_t TOMB_DROP = 240;
		static constexpr int16_t TOMB_SPEED = 2;
		static constexpr float GHOST_FADE = 0.004f;
		static constexpr float GHOST_RADIUS = 9.0f;
		static constexpr uint16_t GHOST_LAP = 2000 / Constants::TIMESTEP;
		Animation tomb;
		int16_t tomb_drop = 0;
		bool tomb_landed = false;
		Linear<float> ghost_opacity;
		uint16_t ghost_tick = 0;

		static EnumMap<CharEffect::Id, Animation> chareffects;
		static Animation tomb_fall;
		static Animation tomb_land;
	};
}