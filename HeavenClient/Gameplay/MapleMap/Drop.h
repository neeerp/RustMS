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
#include <vector>

#include "MapObject.h"

#include "../Template/Interpolated.h"
#include "../Template/Rectangle.h"

namespace ms
{
	class Drop : public MapObject
	{
	public:
		virtual int8_t update(const Physics &physics) override;

		void expire(int8_t, const PhysicsObject *);

		// Whether the player with these feet, id and party could pick the
		// drop up now.
		bool lootable(Point<int16_t> feet, int32_t self, int32_t party) const;

		// Note a pickup request, so the drop isn't tried again for a while.
		void try_loot();

		// Whether a pet at `feet` could take the drop now, if it takes mesos,
		// items and others' drops as given and leaves the ids `ignored`
		// (FUN_0050483a).
		bool pet_lootable(Point<int16_t> feet, int32_t self, int32_t party, bool mesos, bool items, bool others,
						  const std::vector<int32_t> &ignored) const;

		// Note a pet's request, so no pet tries the drop again for a while.
		void try_pet_loot();

		// Whether a pet in Wing Boots at `pet`, whose owner is at `ownerpos`,
		// would walk toward the drop, if it takes items and others' drops
		// as given and leaves the ids `ignored` (FUN_00508268). Any pet
		// walks toward mesos. Unlike looting, this notes no try.
		bool pet_sweepable(Point<int16_t> ownerpos, Point<int16_t> pet, int32_t self, int32_t party, bool items,
						   bool others, bool longrange, const std::vector<int32_t> &ignored) const;

		// The item, or 0 for mesos.
		virtual int32_t get_itemid() const;

		// How many mesos it is, or 0 for an item.
		virtual int32_t get_mesos() const;

		// Where the server put it.
		Point<int16_t> get_spot() const;

		// Whether the player with this id and party could set it off with
		// Meso Explosion from `area` (FUN_0050579e).
		bool explodable(Rectangle<int16_t> area, int32_t self, int32_t party) const;

		// Leave it where it lies, for nobody to take or set off again, until
		// it goes off in `delay` ms (FUN_00506590).
		void explode(int16_t delay);

		// Whether it went off this update. It is gone the next.
		bool went_off() const;

	protected:
		Drop(int32_t oid, int32_t owner, Point<int16_t> start, Point<int16_t> dest,
			 int8_t type, int8_t mode, int16_t delay, bool playerdrop, bool spins);

		// Where to draw an icon so it is centred on the drop, as the original
		// centres a drop's layer on its position.
		static Point<int16_t> centred(Point<int16_t> position, Point<int16_t> origin, Point<int16_t> dimensions);

		Linear<float> opacity;
		Linear<float> angle;

	private:
		enum State
		{
			WAITING,
			DROPPED,
			FALLING,
			FLOATING,
			PICKEDUP,
			EXPIRING,
			EXPLODING,
			EXPLODED
		};

		// Fly toward the looter's middle.
		void pick_up();
		// Move to a position computed for this tick.
		void move_to(const Physics &physics, double x, double y);
		// An item nobody may pick up (`pickUpBlock`), which v83 leaves out
		// of both the pickup key and pets' looting.
		bool blocked() const;

		int32_t owner;
		int8_t pickuptype;
		int8_t mode;
		bool playerdrop;
		bool spins;

		const PhysicsObject *looter;
		// It flies to a pet's feet rather than to a character's middle.
		bool topet;
		State state;

		Point<int16_t> start;
		Point<int16_t> dest;
		// Where the server put the drop
		Point<int16_t> spot;
		// Milliseconds spent in the current state.
		int32_t elapsed;
		// Milliseconds since the drop arrived, and since the last pickup
		// request for it.
		int32_t age;
		int32_t sincetry;
		int32_t sincepet;
		int32_t delay;
		// Milliseconds until it goes off, once set to.
		int32_t fuse;
		int32_t flighttime;
		double phase;
		Point<int16_t> pickupstart;
	};
}
