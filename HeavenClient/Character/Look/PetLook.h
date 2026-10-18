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

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../Template/EnumMap.h"

#include "../../Gameplay/MovePath.h"
#include "../../Gameplay/Physics/Physics.h"
#include "../../Net/Handlers/Helpers/PetParser.h"

#include <optional>

namespace ms
{
	class PetLook
	{
	public:
		enum Stance : uint8_t
		{
			MOVE,
			STAND,
			JUMP,
			ALERT,
			PRONE,
			FLY,
			HANG,
			WARP,
			LENGTH
		};

		// The animation a v83 move action shows, of those HC loads.
		static Stance stancebyvalue(uint8_t value);

		// The player's own pet (`local`) follows them and says where it
		// went; another's goes where its owner's client says.
		PetLook(const PetEntry &entry, bool local);

		PetLook();

		void draw(double viewx, double viewy, float alpha) const;

		// Follow the owner at `charpos`, returning a path to send once
		// there is one, or play the paths received.
		std::optional<move_path::Path> update(const Physics &physics, Point<int16_t> charpos);

		// MOVE_PET (170): where another's pet went.
		void send_movement(const std::vector<Movement> &movements);

		void set_position(int16_t xpos, int16_t ypos);

		void set_stance(Stance stance);

		void set_stance(uint8_t stancebyte);

		int32_t get_itemid() const;

		int64_t get_serial() const;

		bool is_local() const;

		Stance get_stance() const;

	private:
		// Its stance as v83 sends it: the move action times two, plus one
		// facing left.
		uint8_t get_stancebyte() const;

		int32_t itemid;
		std::string name;
		int64_t serial;
		bool local;
		Stance stance;
		// Facing right.
		bool flip;

		EnumMap<Stance, Animation> animations;
		PhysicsObject phobj;
		Text namelabel;

		// The owner's: the path not yet sent.
		move_path::Recorder recorder;
		// The others': the paths to play.
		move_path::Player shown;
	};
}