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

#include "Body.h"
#include "CharEquips.h"
#include "Face.h"
#include "Hair.h"

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

		bool update(uint16_t timestep);

		void set_hair(int32_t hairid);

		void set_body(int32_t skinid);

		void set_face(int32_t faceid);

		void add_equip(int32_t equipid);
		void remove_equip(EquipSlot::Id slot);

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

		// The delay of each frame of a body action, in order.
		std::vector<uint16_t> get_action_delays(const std::string &action) const;

		// How long a skill's body action keeps the user from acting.
		uint16_t get_action_lock(const std::string &action) const;

		uint8_t get_frame() const;

		Stance::Id get_stance() const;

		// Return the face's offset from the feet in the current frame, facing left.
		Point<int16_t> get_facepos() const;

		const Body *get_body() const;

		const Hair *get_hair() const;

		const Face *get_face() const;

		const CharEquips &get_equips() const;

		// Initialize drawinfo
		static void init();

	private:
		void updatetwohanded();
		void update_expression(uint16_t timestep);

		void draw(const DrawArgument &args, Stance::Id interstance, Expression::Id interexp, uint8_t interframe,
				  uint8_t interfcframe) const;

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

		Randomizer randomizer;
		TimedBool alerted;

		static BodyDrawInfo drawinfo;
		static std::unordered_map<int32_t, Hair> hairstyles;
		static std::unordered_map<int32_t, Face> facetypes;
		static std::unordered_map<int32_t, Body> bodytypes;
	};
}