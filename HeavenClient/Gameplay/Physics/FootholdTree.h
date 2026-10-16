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

#include "Foothold.h"
#include "PhysicsObject.h"

#include <unordered_map>
#include <vector>

namespace ms
{
	// The collection of platforms in a maple map
	// Used for collision-detection
	class FootholdTree
	{
	public:
		// Where a rising wall is in front of a walking object.
		enum class WallAhead
		{
			NONE,
			NEAR,
			AGAINST
		};

		// info is the map's info node, whose VRLimit can narrow the bounds.
		FootholdTree(nl::node source, nl::node info = {});

		FootholdTree();

		// Takes an accelerated PhysicsObject and limits its movement based on the platforms in this tree.
		void limit_movement(PhysicsObject &touse) const;

		// Keeps a flier within the map's physical space.
		void limit_flight(PhysicsObject &touse) const;

		// Updates a PhysicsObject's fhid based on it's position.
		void update_fh(PhysicsObject &touse) const;

		// Puts a PhysicsObject on the platform the server says it stands on.
		void place_on_fh(PhysicsObject &touse, uint16_t fhid) const;

		// Determine the point on the ground below the specified position.
		int16_t get_y_below(Point<int16_t> position) const;

		// Returns the leftmost and rightmost positions objects may reach.
		Range<int16_t> get_walls() const;

		// Returns the highest position objects may reach and the lowest they
		// may fall to.
		Range<int16_t> get_borders() const;

		// Returns the group of a platform.
		uint16_t get_group(uint16_t fhid) const;

		// Returns the group of the platform at or below a position, 0 if
		// there is none.
		uint16_t get_group_below(Point<int16_t> position) const;

		// Returns the leftmost and rightmost x of a group's platforms, or an
		// empty range for a group the map doesn't have.
		Range<int16_t> get_group_reach(uint16_t group) const;

		// Returns the map's groups, in the order it lists them.
		const std::vector<uint16_t> &get_groups() const;

		// Returns the platforms of a group, walls included.
		const std::vector<uint16_t> &get_group_footholds(uint16_t group) const;

		// Returns a platform, or an empty one for an id the map doesn't have.
		const Foothold &get_fh(uint16_t fhid) const;

		// Looks for a rising wall up to 40 px ahead of a grounded object
		// walking in direction dir, and whether the object is against it.
		WallAhead get_wall_ahead(const PhysicsObject &phobj, int8_t dir) const;

	private:
		bool limit_jumper(PhysicsObject &phobj) const;

		uint16_t get_fhid_below(double fx, double fy, uint16_t ignore = 0) const;

		bool can_jump_down(uint16_t fhid, double fx, double fy) const;

		double get_wall(uint16_t fhid, bool left, double fy) const;

		double get_edge(uint16_t fhid, bool left) const;

		double get_walk_edge(uint16_t fhid, bool left) const;

		std::unordered_map<uint16_t, Foothold> footholds;
		std::unordered_multimap<int16_t, uint16_t> footholdsbyx;
		std::unordered_map<uint16_t, Range<int16_t>> groups;
		std::vector<uint16_t> grouporder;
		std::unordered_map<uint16_t, std::vector<uint16_t>> groupfootholds;

		Foothold nullfh;
		Range<int16_t> walls;
		Range<int16_t> borders;
		Range<int16_t> spacex;
		Range<int16_t> spacey;
	};
}