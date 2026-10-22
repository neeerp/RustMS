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
#include <cstdint>
#include <optional>

#include "MapObjects.h"

#include "../Spawn.h"

#include "../Graphics/Animation.h"

#include "../Combat/MesoExplosion.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/EffectLayer.h"

#include <array>
#include <queue>
#include <vector>

namespace ms
{
	class MapDrops
	{
	public:
		MapDrops();

		// Initialize the meso icons.
		void init();

		// Draw all drops on a layer.
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// Draw the mesos going off, on the effect layer (z 0xc00614a4).
		void draw_explosions(Point<int16_t> viewpos, float alpha) const;

		// Update all drops.
		void update(const Physics &physics);

		// Spawn a new drop.
		void spawn(DropSpawn &&spawn);

		// Remove a drop.
		void remove(int32_t oid, int8_t mode, const PhysicsObject *looter);

		// Set a drop to go off in `delay` ms, for Meso Explosion (mode 4).
		void explode(int32_t oid, int16_t delay);

		// The mesos in `area` the player with this id and party could set
		// off with Meso Explosion, in the order they entered the map
		// (FUN_0050579e).
		std::vector<meso_explosion::Drop> find_explodable(Rectangle<int16_t> area, int32_t self, int32_t party) const;

		// Remove all drops.
		void clear();

		// Find a drop the player with these feet, id and party can pick
		// up, and note the try. As in v83, the first one that qualifies.
		using Loot = std::pair<int32_t, Point<int16_t>>;

		Loot find_loot_at(Point<int16_t> playerpos, int32_t self, int32_t party);

		// The first drop a pet at `petpos` would reach for, if it takes
		// mesos, items and others' drops as given and leaves the ids
		// `ignored`, or 0 for none (FUN_0050483a).
		int32_t find_pet_loot_at(Point<int16_t> petpos, int32_t self, int32_t party, bool mesos, bool items,
								 bool others, const std::vector<int32_t> &ignored);

		// A pet out in Wing Boots, as the sweep sees it: where it is, whether
		// it takes items and others' drops, whether it wears a Binocular,
		// and the ids it leaves.
		struct Sweeper
		{
			Point<int16_t> position;
			bool items;
			bool others;
			bool longrange;
			const std::vector<int32_t> *ignored;
		};

		// Which way each of `pets` walks toward a drop, 0 for none
		// (FUN_00508268): drop by drop, each goes to the nearest pet that
		// would walk to it and isn't walking to another.
		std::array<int8_t, 3> sweep(Point<int16_t> owner, const std::array<std::optional<Sweeper>, 3> &pets,
									int32_t self, int32_t party);

	private:
		MapObjects drops;

		enum MesoIcon
		{
			BRONZE,
			GOLD,
			BUNDLE,
			BAG,
			NUM_ICONS
		};

		std::array<Animation, MesoIcon::NUM_ICONS> mesoicons;

		// What a drop going off shows, one picked at random, and sounds like
		// (FUN_00504bff).
		std::vector<Animation> blasts;
		Sound blastsound;
		int32_t sinceblast;
		EffectLayer explosions;

		std::queue<DropSpawn> spawns;
	};
}