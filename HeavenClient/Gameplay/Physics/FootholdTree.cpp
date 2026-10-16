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
#include "FootholdTree.h"

#include <algorithm>
#include <cmath>
#include <iostream>

namespace ms
{
	FootholdTree::FootholdTree(nl::node src, nl::node info)
	{
		int16_t leftw = 30000;
		int16_t rightw = -30000;
		int16_t botb = -30000;
		int16_t topb = 30000;

		for (auto basef : src)
		{
			uint8_t layer;

			try
			{
				layer = static_cast<uint8_t>(std::stoi(basef.name()));
			}
			catch (const std::exception &ex)
			{
				std::cout << __func__ << ": " << ex.what() << std::endl;
				continue;
			}

			for (auto midf : basef)
			{
				uint16_t group;

				try
				{
					group = static_cast<uint16_t>(std::stoi(midf.name()));
				}
				catch (const std::exception &ex)
				{
					std::cout << __func__ << ": " << ex.what() << std::endl;
					continue;
				}

				for (auto lastf : midf)
				{
					uint16_t id;

					try
					{
						id = static_cast<uint16_t>(std::stoi(lastf.name()));
					}
					catch (const std::exception &ex)
					{
						std::cout << __func__ << ": " << ex.what() << std::endl;
						continue;
					}

					const Foothold &foothold = footholds.emplace(
							std::piecewise_construct,
							std::forward_as_tuple(id),
							std::forward_as_tuple(lastf, id, layer, group)
					).first->second;

					// v83 (FUN_0085fe71) keeps how far each group reaches, walls
					// included.
					auto [extent, added] = groups.try_emplace(group, foothold.l(), foothold.r());

					if (added)
						grouporder.push_back(group);

					groupfootholds[group].push_back(id);
					extent->second = {
						std::min(extent->second.first(), foothold.l()),
						std::max(extent->second.second(), foothold.r())
					};

					if (foothold.l() < leftw)
						leftw = foothold.l();

					if (foothold.r() > rightw)
						rightw = foothold.r();

					if (foothold.b() > botb)
						botb = foothold.b();

					if (foothold.t() < topb)
						topb = foothold.t();

					if (foothold.is_wall())
						continue;

					int16_t start = foothold.l();
					int16_t end = foothold.r();

					for (int16_t i = start; i <= end; i++)
						footholdsbyx.emplace(i, id);
				}
			}
		}

		// v83 (FUN_0085fe71) keeps objects 30 px inside the outermost
		// footholds and up to 300 px above the highest. A map with VRLimit
		// pulls those in to its view range; like v83, it reads a missing edge
		// as 0 and skips an edge of 0. v83 doesn't stop a fall below the
		// lowest foothold; this stops it 100 px below.
		int16_t left = leftw + 30;
		int16_t right = rightw - 30;
		int16_t top = topb - 300;

		if (info["VRLimit"].get_bool())
		{
			if (int16_t vrleft = info["VRLeft"])
				left = std::max<int16_t>(left, vrleft + 20);

			if (int16_t vrright = info["VRRight"])
				right = std::min<int16_t>(right, vrright - 20);

			if (int16_t vrtop = info["VRTop"])
				top = std::max<int16_t>(top, vrtop + 65);
		}

		walls = {left, right};
		borders = {top, static_cast<int16_t>(botb + 100)};
		spacex = {left, right};
		int16_t bottom = botb + 10;

		if (info["VRLimit"].get_bool())
			if (int16_t vrbottom = info["VRBottom"])
				bottom = std::min(bottom, vrbottom);

		spacey = {top, bottom};
	}

	FootholdTree::FootholdTree()
	{}

	void FootholdTree::limit_movement(PhysicsObject &phobj) const
	{
		bool jumper = phobj.is_flag_set(PhysicsObject::Flag::JUMPER) &&
					  phobj.is_flag_set(PhysicsObject::Flag::TURNATEDGES);

		if (phobj.hmobile() && !(jumper && phobj.onground && limit_jumper(phobj)))
		{
			double crnt_x = phobj.crnt_x();
			double next_x = phobj.next_x();

			bool left = phobj.hspeed < 0.0f;
			double wall = get_wall(phobj.fhid, left, phobj.next_y());
			bool collision = left ? crnt_x >= wall && next_x <= wall : crnt_x <= wall && next_x >= wall;

			if (!collision && phobj.is_flag_set(PhysicsObject::Flag::TURNATEDGES) && !jumper)
			{
				if (phobj.is_flag_set(PhysicsObject::Flag::WALKONLY))
				{
					// v83 checks this only while walking, and pulls the object
					// back to the limit even if it started past it, e.g. when it
					// spawned near the end of its foothold.
					if (phobj.onground)
					{
						wall = get_walk_edge(phobj.fhid, left);
						collision = left ? next_x < wall : next_x > wall;
					}
				} else
				{
					wall = get_edge(phobj.fhid, left);
					collision = left ? crnt_x >= wall && next_x <= wall : crnt_x <= wall && next_x >= wall;
				}
			}

			if (collision)
			{
				phobj.limitx(wall);

				// A jumper only stops at a foothold wall, which it may jump
				// next, but turns at the edge of the map.
				bool mapedge = wall == (left ? walls.first() : walls.second());

				if (!jumper || mapedge)
					phobj.clear_flag(PhysicsObject::Flag::TURNATEDGES);
			}
		}

		if (phobj.vmobile())
		{
			double crnt_y = phobj.crnt_y();
			double next_y = phobj.next_y();

			auto ground = Range<double>(
					get_fh(phobj.fhid).ground_below(phobj.crnt_x()),
					get_fh(phobj.fhid).ground_below(phobj.next_x())
			);

			bool collision = crnt_y <= ground.first() && next_y >= ground.second();

			if (collision)
			{
				phobj.limity(ground.second());
				// The object has landed. Keeping the old airborne flag until the
				// next tick drops landing-time jump input and adds a friction-only
				// tick before the walking controller resumes.
				phobj.onground = true;
				phobj.fallstart = 0;

				limit_movement(phobj);
			} else
			{
				if (next_y < borders.first())
					phobj.limity(borders.first());
				else if (next_y > borders.second())
					phobj.limity(borders.second());
			}
		}
	}

	void FootholdTree::limit_flight(PhysicsObject &phobj) const
	{
		// FUN_007f9202 moves a flier back inside, leaving its speed alone.
		phobj.x = std::clamp(phobj.crnt_x(), double(spacex.first()), spacex.second() - 1.0);
		phobj.y = std::clamp(phobj.crnt_y(), double(spacey.first()), spacey.second() - 1.0);
	}

	void FootholdTree::update_fh(PhysicsObject &phobj) const
	{
		if (phobj.type == PhysicsObject::Type::FIXATED && phobj.fhid > 0)
			return;

		const Foothold &curfh = get_fh(phobj.fhid);
		bool checkslope = false;

		double x = phobj.crnt_x();
		double y = phobj.crnt_y();

		if (phobj.onground)
		{
			if (std::floor(x) > curfh.r())
				phobj.fhid = curfh.next();
			else if (std::ceil(x) < curfh.l())
				phobj.fhid = curfh.prev();

			if (phobj.fhid == 0)
				phobj.fhid = get_fhid_below(x, y);
			else
				checkslope = true;
		} else
		{
			phobj.fhid = get_fhid_below(x, y, phobj.fallstart);
		}

		const Foothold &nextfh = get_fh(phobj.fhid);
		phobj.fhslope = nextfh.slope();

		double ground = nextfh.ground_below(x);

		if (phobj.vspeed == 0.0 && checkslope && phobj.fhid == curfh.id())
		{
			// v83 moves a grounded object along its foothold (FUN_007f13de),
			// so it stays on the line, even on the tick it stops on a slope.
			phobj.y = ground;
		} else if (phobj.vspeed == 0.0 && checkslope)
		{
			double vdelta = abs(phobj.fhslope);

			if (phobj.fhslope < 0.0)
				vdelta *= (ground - y);
			else if (phobj.fhslope > 0.0)
				vdelta *= (y - ground);

			if (curfh.slope() != 0.0 || nextfh.slope() != 0.0)
			{
				if (phobj.hspeed > 0.0 && vdelta <= phobj.hspeed)
					phobj.y = ground;
				else if (phobj.hspeed < 0.0 && vdelta >= phobj.hspeed)
					phobj.y = ground;
			}
		}

		phobj.onground = phobj.y == ground;

		if (phobj.onground)
			phobj.fallstart = 0;

		if (phobj.enablejd || phobj.is_flag_set(PhysicsObject::Flag::CHECKBELOW))
		{
			phobj.enablejd = can_jump_down(phobj.fhid, x, ground);
			phobj.groundbelow = ground + 1.0;
			phobj.clear_flag(PhysicsObject::Flag::CHECKBELOW);
		}

		if (phobj.fhlayer == 0 || phobj.onground)
			phobj.fhlayer = nextfh.layer();

		if (phobj.fhid == 0)
		{
			phobj.fhid = curfh.id();
			phobj.limitx(curfh.x1());
		}
	}

	const Foothold &FootholdTree::get_fh(uint16_t fhid) const
	{
		auto iter = footholds.find(fhid);

		if (iter == footholds.end())
			return nullfh;

		return iter->second;
	}

	double FootholdTree::get_wall(uint16_t curid, bool left, double fy) const
	{
		auto shorty = static_cast<int16_t>(fy);
		Range<int16_t> vertical(shorty - 50, shorty - 1);
		const Foothold &cur = get_fh(curid);

		if (left)
		{
			const Foothold &prev = get_fh(cur.prev());

			if (prev.is_blocking(vertical))
				return cur.l();

			const Foothold &prev_prev = get_fh(prev.prev());

			if (prev_prev.is_blocking(vertical))
				return prev.l();

			return walls.first();
		} else
		{
			const Foothold &next = get_fh(cur.next());

			if (next.is_blocking(vertical))
				return cur.r();

			const Foothold &next_next = get_fh(next.next());

			if (next_next.is_blocking(vertical))
				return next.r();

			return walls.second();
		}
	}

	double FootholdTree::get_edge(uint16_t curid, bool left) const
	{
		const Foothold &fh = get_fh(curid);

		if (left)
		{
			uint16_t previd = fh.prev();

			if (!previd)
				return fh.l();

			const Foothold &prev = get_fh(previd);
			uint16_t prev_previd = prev.prev();

			if (!prev_previd)
				return prev.l();

			return walls.first();
		} else
		{
			uint16_t nextid = fh.next();

			if (!nextid)
				return fh.r();

			const Foothold &next = get_fh(nextid);
			uint16_t next_nextid = next.next();

			if (!next_nextid)
				return next.r();

			return walls.second();
		}
	}

	double FootholdTree::get_walk_edge(uint16_t curid, bool left) const
	{
		// v83 (FUN_007f8d64, move ability 1) lets a mob walk on to the next
		// foothold only if that one runs left to right; at a wall, a ceiling
		// or a loose end it stops 20 px along this foothold from its end.
		constexpr double MARGIN = 20.0;

		const Foothold &fh = get_fh(curid);
		const Foothold &neighbour = get_fh(left ? fh.prev() : fh.next());

		if (neighbour.x2() > neighbour.x1())
			return left ? walls.first() : walls.second();

		double dx = fh.x2() - fh.x1();
		double length = std::hypot(dx, fh.y2() - fh.y1());

		if (length == 0.0)
			return left ? fh.x1() : fh.x2();

		double margin = std::min(MARGIN, length) * dx / length;

		return left ? fh.x1() + margin : fh.x2() - margin;
	}

	bool FootholdTree::limit_jumper(PhysicsObject &phobj) const
	{
		// v83 (FUN_007f8d64, move ability 2) keeps the mob within the reach
		// of its foothold's group and its home group, less 20 px a side when
		// that is wider than 40 px. Leaving it turns the mob around.
		const Foothold &fh = get_fh(phobj.fhid);
		Range<int16_t> crnt = get_group_reach(fh.group());
		Range<int16_t> home = get_group_reach(phobj.homegroup);
		int32_t left = std::min(crnt.first(), home.first());
		int32_t right = std::max(crnt.second(), home.second());

		if (right - left > 40)
		{
			left += 20;
			right -= 20;
		}

		double next_x = phobj.next_x();

		if (next_x < left || next_x > right)
		{
			phobj.limitx(next_x > right ? right : left);
			phobj.clear_flag(PhysicsObject::Flag::TURNATEDGES);

			return true;
		}

		// Walking past the end of its foothold, it carries on onto one that
		// runs left to right, and down a step if that is in reach. At a loose
		// end or any other wall it stops to jump, when edgejump says so.
		bool toleft = phobj.hspeed < 0.0;

		if (toleft ? next_x >= fh.x1() : next_x <= fh.x2())
			return false;

		uint16_t neighbourid = toleft ? fh.prev() : fh.next();

		if (neighbourid)
		{
			const Foothold &neighbour = get_fh(neighbourid);

			if (neighbour.x2() > neighbour.x1())
				return false;

			bool down = toleft ? neighbour.y2() < neighbour.y1() : neighbour.y2() > neighbour.y1();
			bool inreach = toleft ? left < fh.x1() : fh.x2() < right;

			if (down && inreach && phobj.stepdown)
				return false;
		}

		if (!phobj.edgejump)
			return false;

		phobj.limitx(toleft ? fh.x1() : fh.x2());
		phobj.jumprequest = true;

		return true;
	}

	FootholdTree::WallAhead FootholdTree::get_wall_ahead(const PhysicsObject &phobj, int8_t dir) const
	{
		// The jumping mob AI (FUN_007f835e) follows the footholds 40 px
		// ahead (FUN_0085f405), up to a loose end or the first one that
		// doesn't run left to right.
		constexpr double DISTANCE = 40.0;

		const Foothold &crnt = get_fh(phobj.fhid);
		double dx = crnt.x2() - crnt.x1();

		if (dir == 0 || !phobj.onground || dx <= 0.0)
			return WallAhead::NONE;

		double length = std::hypot(dx, crnt.y2() - crnt.y1());
		double along = (phobj.crnt_x() - crnt.x1()) * length / dx;
		double ahead = DISTANCE - (dir > 0 ? length - along : along);
		const Foothold *fh = &crnt;

		while (ahead > 0.0)
		{
			uint16_t id = dir > 0 ? fh->next() : fh->prev();

			if (!id)
				return WallAhead::NONE;

			fh = &get_fh(id);

			if (fh->x2() <= fh->x1())
				break;

			ahead -= std::hypot(fh->x2() - fh->x1(), fh->y2() - fh->y1());
		}

		auto rising = [dir](const Foothold &wall)
		{
			return wall.x2() <= wall.x1() && (dir > 0 ? wall.y2() < wall.y1() : wall.y2() > wall.y1());
		};

		if (fh == &crnt || !rising(*fh))
			return WallAhead::NONE;

		// Pressed against the wall, the AI turns back instead of jumping
		// again.
		bool against = rising(get_fh(dir > 0 ? crnt.next() : crnt.prev())) &&
					   (dir > 0 ? along > length - 1.0 : along < 1.0);

		return against ? WallAhead::AGAINST : WallAhead::NEAR;
	}

	uint16_t FootholdTree::get_group(uint16_t fhid) const
	{
		return get_fh(fhid).group();
	}

	uint16_t FootholdTree::get_group_below(Point<int16_t> position) const
	{
		return get_fh(get_fhid_below(position.x(), position.y() - 1.0)).group();
	}

	Range<int16_t> FootholdTree::get_group_reach(uint16_t group) const
	{
		auto iter = groups.find(group);

		if (iter == groups.end())
			return {INT16_MAX, INT16_MIN};

		return iter->second;
	}

	const std::vector<uint16_t> &FootholdTree::get_groups() const
	{
		return grouporder;
	}

	const std::vector<uint16_t> &FootholdTree::get_group_footholds(uint16_t group) const
	{
		static const std::vector<uint16_t> none;

		auto iter = groupfootholds.find(group);

		return iter == groupfootholds.end() ? none : iter->second;
	}

	uint16_t FootholdTree::get_fhid_below(double fx, double fy, uint16_t ignore) const
	{
		uint16_t ret = 0;
		double comp = borders.second();

		int16_t x = static_cast<int16_t>(fx);
		auto range = footholdsbyx.equal_range(x);

		for (auto iter = range.first; iter != range.second; ++iter)
		{
			const Foothold &fh = footholds.at(iter->second);
			double ycomp = fh.ground_below(fx);

			if (fh.id() != ignore && comp >= ycomp && ycomp >= fy)
			{
				comp = ycomp;
				ret = fh.id();
			}
		}

		return ret;
	}

	void FootholdTree::place_on_fh(PhysicsObject &phobj, uint16_t fhid) const
	{
		// v83 (FUN_007f13de) measures how far along the foothold the object
		// is, by x unless the foothold is steep, clamps that to the foothold
		// and takes the point there.
		const Foothold &fh = get_fh(fhid);
		double dx = fh.x2() - fh.x1();
		double dy = fh.y2() - fh.y1();
		double length = std::hypot(dx, dy);

		if (length == 0.0)
			return;

		double ux = dx / length;
		double uy = dy / length;
		double along = std::abs(ux) > 0.5 ? (phobj.crnt_x() - fh.x1()) / ux : (phobj.crnt_y() - fh.y1()) / uy;
		along = std::clamp(along, 0.0, length);
		phobj.set_x(fh.x1() + ux * along);
		phobj.set_y(fh.y1() + uy * along);
		phobj.fhid = fhid;
		phobj.fhlayer = fh.layer();
		phobj.onground = true;
	}

	// CUserLocal's down-jump check (FUN_007a44fe) looks for the lowest
	// platform at most 300 px under the feet, or failing that the highest one
	// 300 to 600 px under. Only platforms drawn left to right count, and one
	// within 5 px of the feet doesn't.
	bool FootholdTree::can_jump_down(uint16_t fhid, double fx, double fy) const
	{
		uint16_t lowest = 0;
		double lowesty = 0.0;
		uint16_t highest = 0;
		double highesty = fy + 600.0;

		auto range = footholdsbyx.equal_range(static_cast<int16_t>(fx));

		for (auto iter = range.first; iter != range.second; ++iter)
		{
			const Foothold &fh = footholds.at(iter->second);

			if (fh.x1() >= fh.x2())
				continue;

			double ycomp = fh.ground_below(fx);

			if (ycomp <= fy + 300.0 && (!lowest || ycomp > lowesty))
			{
				lowest = fh.id();
				lowesty = ycomp;
			}

			if (ycomp >= fy + 300.0 && ycomp < highesty)
			{
				highest = fh.id();
				highesty = ycomp;
			}
		}

		if (!lowest || lowest == fhid)
		{
			lowest = highest;
			lowesty = highesty;
		}

		return lowest && std::abs(lowesty - fy) >= 5.0;
	}

	int16_t FootholdTree::get_y_below(Point<int16_t> position) const
	{
		if (uint16_t fhid = get_fhid_below(position.x(), position.y()))
		{
			const Foothold &fh = get_fh(fhid);

			return static_cast<int16_t>(fh.ground_below(position.x()));
		} else
		{
			return borders.second();
		}
	}

	Range<int16_t> FootholdTree::get_walls() const
	{
		return walls;
	}

	Range<int16_t> FootholdTree::get_borders() const
	{
		return borders;
	}
}
