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
#include "Obj.h"

#include "ObjStateRules.h"

#include "../FieldEffectRules.h"

#include "../../Graphics/GraphicsGL.h"
#include "../../Util/Misc.h"

#include <cmath>
#include <set>

#include <nlnx/nx.hpp>

namespace ms
{
	Obj::State::State(nl::node data) : animation(data)
	{
		repeat = data["repeat"].get_integer();

		// Only the coconuts have one, from Sound.nx Field.img (FUN_0098a563).
		if (nl::node sfx = data["sfx"])
			sound = Sound(nl::nx::sound["Field.img"].resolve(sfx.get_string()));

		// v83 FUN_0063c741.
		obstacle = data["obstacle"].get_integer() != 0;

		if (obstacle)
		{
			trap.damage = data["damage"];
			trap.mobdamage = data["mobdamage"];
			trap.dir = data["dir"];
			trap.disease = data["disease"];
			trap.level = data["level"];
			trap.target_field = data["targetField"];

			if (data["forceHP"])
				trap.force_hp = data["forceHP"];

			// In the order Animation keeps its frames.
			std::set<int16_t> frameids;

			for (auto sub : data)
				if (sub.data_type() == nl::node::type::bitmap)
					if (int16_t fid = string_conversion::or_default<int16_t>(sub.name(), -1); fid >= 0)
						frameids.insert(fid);

			for (int16_t fid : frameids)
				frame_dirs.push_back(data[std::to_string(fid)]["dir"]);
		}

		// Nothing animates until it is played.
		animation.stop();
	}

	Obj::Obj(nl::node src, int32_t order) : current(0), order(order)
	{
		nl::node data = nl::nx::map["Obj"][src["oS"] + ".img"][src["l0"]][src["l1"]][src["l2"]];

		name = src["name"].get_string();
		tags = obj_state::split_tags(src["tags"].get_string());
		shown = !src["hide"].get_bool();
		pos = Point<int16_t>(src["x"], src["y"]);
		flip = src["f"].get_bool();
		z = src["z"];
		additive = data["blend"].get_integer() == 1;

		states.emplace_back(data);

		// A named obj's states are hidden until the server shows one, except for an autoActive one's first
		// (FUN_0063ad16).
		if (name.empty())
		{
			visible = true;
		} else
		{
			for (int32_t i = 1; nl::node sub = data["s" + std::to_string(i)]; i++)
				states.emplace_back(sub);

			visible = data["autoActive"].get_integer() != 0;
		}

		if (visible)
			play(states[0]);
	}

	void Obj::play(State &state)
	{
		obj_state::Playback playback = obj_state::playback(state.repeat);

		if (playback.animates && !(playback.waits && state.animation.is_playing()))
			state.animation.play(playback.end, playback.loop_from);
	}

	void Obj::update()
	{
		if (visible)
			states[current].animation.update();
	}

	void Obj::draw(Point<int16_t> viewpos, float inter) const
	{
		if (!visible || !shown)
			return;

		if (additive)
			GraphicsGL::get().set_additive(true);

		states[current].animation.draw(DrawArgument(pos + viewpos, flip), inter);

		if (additive)
			GraphicsGL::get().set_additive(false);
	}

	const std::string &Obj::get_name() const
	{
		return name;
	}

	bool Obj::set_state(int32_t state, Point<int16_t> listener)
	{
		int32_t next = obj_state::next_state(static_cast<int32_t>(current), static_cast<int32_t>(states.size()), state);

		if (next < 0)
			return false;

		// The state it leaves stops; switching to the same one starts it again.
		if (state != -1)
			states[current].animation.stop();

		current = next;
		visible = true;

		State &shown = states[current];
		Point<int16_t> distance = pos - listener;
		double length = std::sqrt(static_cast<double>(distance.x()) * distance.x() + static_cast<double>(distance.y()) * distance.y());

		shown.sound.play(field_effect::summon_volume(length));
		play(shown);

		return true;
	}

	const std::vector<std::string> &Obj::get_tags() const
	{
		return tags;
	}

	void Obj::set_shown(bool value)
	{
		shown = value;
	}

	bool Obj::has_obstacle() const
	{
		for (auto &state : states)
			if (state.obstacle)
				return true;

		return false;
	}

	uint8_t Obj::getz() const
	{
		return z;
	}

	int32_t Obj::get_order() const
	{
		return order;
	}

	// A hidden state doesn't hurt (FUN_00642264 skips a layer below alpha 126).
	bool Obj::is_obstacle() const
	{
		return visible && states[current].obstacle;
	}

	const obstacle::Trap &Obj::get_trap() const
	{
		return states[current].trap;
	}

	bool Obj::obstacle_box(obstacle::Box &out) const
	{
		Rectangle<int16_t> bounds = states[current].animation.get_bounds();
		Point<int16_t> lt = bounds.get_left_top();
		Point<int16_t> rb = bounds.get_right_bottom();

		return obstacle::box(pos.x(), pos.y(), { lt.x(), lt.y() }, { rb.x(), rb.y() }, flip, out);
	}

	obstacle::Vec Obj::get_push() const
	{
		const State &state = states[current];
		int32_t dir = state.trap.dir;

		// Where the node has no dir, v83 takes the frame's (FUN_00642264), as Sleepywood's spears have.
		if (dir == 0)
		{
			size_t frame = state.animation.get_frame_index(1.0f);

			if (frame < state.frame_dirs.size())
				dir = state.frame_dirs[frame];
		}

		return obstacle::push(dir, flip);
	}
}