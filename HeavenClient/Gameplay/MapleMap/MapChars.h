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

#include "../Movement.h"
#include "../Spawn.h"

#include "../Character/OtherChar.h"
#include "../Character/Party.h"
#include "../Graphics/Geometry.h"

#include <queue>

namespace ms
{
	// A collection of remote controlled characters on a map.
	class MapChars
	{
	public:
		// Draw all characters on a layer.
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// Update all characters.
		void update(const Physics &physics);

		// Spawn a new character, if it has not been spawned yet.
		void spawn(CharSpawn &&spawn);

		// Remove a character.
		void remove(int32_t cid);

		// Remove all characters.
		void clear();

		// Returns a reference to the MapObjects object.
		MapObjects *get_chars();

		// How many other characters are on the map.
		size_t size() const;

		// Update a characters movement.
		void send_movement(int32_t cid, const std::vector<Movement> &movements);

		// Update a characters look, and the ring blocks with it.
		void update_look(int32_t cid, const LookEntry &look, const RingLooks &ring_looks);

		// The characters with a ring block of a kind, where they stand.
		std::vector<RingWearer> ring_wearers(RingKind kind) const;
		// Show each character its pair's ring effect of a kind, and the others none.
		void show_rings(RingKind kind, const std::vector<std::pair<int32_t, RingEffect>> &effects);

		Optional<OtherChar> get_char(int32_t cid);

		// Draw HP bars over the heads of the player's party members.
		void draw_party_hp(const Party &party, int32_t self, int32_t mapid, double viewx, double viewy, float alpha) const;

	private:
		// The bar's top is this far above a member's feet, as in period footage.
		static constexpr int16_t PARTY_HP_RISE = 84;

		// Add the characters spawned since, so packets that follow a spawn find them as v83's do.
		void add_spawns();

		MapObjects chars;
		MobHpBar party_hpbar;

		std::queue<CharSpawn> spawns;
	};
}