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

#include <cstddef>

#include <cstdint>

#include "Body.h"
#include "CharEquips.h"
#include "Face.h"
#include "Hair.h"
#include "TamingMob.h"

#include "../../Net/Login.h"
#include "../../Template/Interpolated.h"
#include "../../Util/Randomizer.h"
#include "../../Util/TimedBool.h"

namespace ms
{
	class CharLook
	{
	public:
		CharLook(const LookEntry &entry);

		CharLook();

		void reset();

		void draw(const DrawArgument &args, float alpha) const;

		void draw(Point<int16_t> pos, bool flip, Stance::Id stance, Expression::Id expression) const;

		// Draw a still pose, on the mount if riding one.
		void draw(const DrawArgument &args, Stance::Id stance, Expression::Id expression) const;

		bool update(uint16_t timestep);

		void set_hair(int32_t hairid);

		void set_body(int32_t skinid);

		void set_face(int32_t faceid);

		void add_equip(int32_t equipid);
		void remove_equip(EquipSlot::Id slot);

		// The clothes the pet at `index` in the list wears (body parts 14,
		// 30 and 38), 0 for none.
		int32_t get_pet_wear(size_t index) const;

		// Put pet equipment `itemid` on body part `part` (14 or 21 to 48),
		// or take it off for 0.
		void set_pet_part(int16_t part, int32_t itemid);

		// The pet equipment worn, by body part.
		const std::map<int16_t, int32_t> &get_pet_parts() const;

		void attack(bool degenerate);

		// Change to a regular attack stance from get_attackstance.
		void attack(bool degenerate, Stance::Id stance);

		void attack(Stance::Id stance);

		// Pick a stance for the weapon's regular attack at random.
		Stance::Id get_attackstance(bool degenerate) const;

		void set_stance(Stance::Id stance);

		// Hold the attack on this frame until end_attack, as v83 holds a
		// held skill's prepare action (0x4547eb).
		void hold(uint8_t frame);

		// Stop the attack, held or not, where it is.
		void end_attack();

		// Show a face, held for this many ms if hold is above 0, looping
		// its animation, as v83 holds the hit face (FUN_00451cf6).
		void set_expression(Expression::Id expression, int32_t hold = 0);

		void set_action(const std::string &action);

		void set_direction(bool mirrored);

		void set_alerted(int64_t millis);

		bool get_alerted() const;

		bool is_twohanded(Stance::Id stance) const;

		uint16_t get_attackdelay(size_t no, uint8_t first_frame) const;

		// How long the attack under way plays, in ms: its body action, or
		// its stance's frames.
		uint16_t get_attack_length() const;

		// The delay of each frame of a body action, in order.
		std::vector<uint16_t> get_action_delays(const std::string &action) const;

		// How long a skill's body action keeps the user from acting.
		uint16_t get_action_lock(const std::string &action) const;

		uint8_t get_frame() const;

		Stance::Id get_stance() const;

		// The body action playing, or else the stance, by name.
		std::string get_action_name() const;

		// Return the face's offset from the feet in the current frame, facing left.
		Point<int16_t> get_facepos() const;

		const Body *get_body() const;

		const Hair *get_hair() const;

		const Face *get_face() const;

		const CharEquips &get_equips() const;

		// Ride a mount (190xxxx or 193xxxx), or get off with 0 (report 201).
		void set_vehicle(int32_t vehicle);

		int32_t get_vehicle() const;

		// Whether the mount stands tired (SET_TAMING_MOB_INFO).
		void set_tired(bool tired);

		// Initialize drawinfo
		static void init();

	private:
		void updatetwohanded();
		void update_expression(uint16_t timestep);

		void draw(const DrawArgument &args, Stance::Id interstance, Expression::Id interexp, uint8_t interframe,
				  uint8_t interfcframe) const;

		// Draw the mount, and the rider sitting on it (FUN_00401de9).
		void draw_mounted(const DrawArgument &args, Stance::Id interstance, Expression::Id interexp,
						  uint8_t interframe, uint8_t interexpframe) const;

		// Whether the mount shows in this stance: a dead rider is a ghost without one.
		bool riding(Stance::Id stance) const;

		// The mount drawn: the one worn, or its cash cover, for a tamed monster, and the ride's for a vehicle.
		int32_t get_mount() const;

		// The saddle drawn, 0 for none.
		int32_t get_saddle() const;

		// The mount's action for the rider's stance (FUN_004138be).
		std::string mount_action(Stance::Id stance) const;

		// The rider's pose and its frame for the mount's frame (FUN_00407757), and how far the rider is moved
		// to sit on the mount's navel.
		Stance::Id rider_pose(Stance::Id stance) const;
		uint8_t rider_frame(Stance::Id pose, uint8_t frame) const;
		Point<int16_t> rider_shift(Stance::Id stance, uint8_t frame) const;

		// Draw the hair over the head and the cap, which covers some of it.
		void draw_cap(const DrawArgument &args, Stance::Id stance, uint8_t frame) const;

		uint16_t get_delay(Stance::Id stance, uint8_t frame) const;

		uint8_t getnextframe(Stance::Id stance, uint8_t frame) const;

		Stance::Id getattackstance(uint8_t attack, bool degenerate) const;

		Nominal<Stance::Id> stance;
		Nominal<uint8_t> stframe;
		uint16_t stelapsed;

		Nominal<Expression::Id> expression;
		Nominal<uint8_t> expframe;
		uint16_t expelapsed;
		int32_t exphold = 0;

		bool flip;

		const BodyAction *action;
		std::string actionstr;
		uint8_t actframe;
		int16_t heldframe = -1;

		const Body *body;
		const Hair *hair;
		const Face *face;
		CharEquips equips;
		std::array<int32_t, 3> pet_wear = {};
		std::map<int16_t, int32_t> pet_parts;
		int32_t vehicle = 0;
		bool tired = false;

		Randomizer randomizer;
		TimedBool alerted;

		static BodyDrawInfo drawinfo;
		static std::unordered_map<int32_t, Hair> hairstyles;
		static std::unordered_map<int32_t, Face> facetypes;
		static std::unordered_map<int32_t, Body> bodytypes;
	};
}