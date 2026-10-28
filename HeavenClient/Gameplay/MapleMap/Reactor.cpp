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
#include "Reactor.h"

#include "../../Constants.h"

#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Reactor::Reactor(int32_t o, int32_t r, int8_t s, Point<int16_t> p) : MapObject(o, p), rid(r), state(s), previous(s),
		phase(Phase::IDLE), deadline(0), cooldown(800), event(UNKNOWN_EVENT), flags(0)
	{
		std::string strid = string_format::extend_id(rid, 7);
		src = nl::nx::reactor[strid + ".img"];

		// Some reactors take their states from another.
		nl::node link = src["info"]["link"];

		if (link.data_type() == nl::node::type::string)
			src = nl::nx::reactor[link.get_string() + ".img"];

		animation = src[std::to_string(state)];
		shift = Animation(src["0"]).get_origin().y();

		// v83 formats the path with "%d/%d", so the id has no leading zeros
		// (0x6345e0).
		nl::node sounds = nl::nx::sound["Reactor.img"][std::to_string(rid)];

		for (int8_t i = 0; sounds[std::to_string(i)]; i++)
		{
			nl::node hit = sounds[std::to_string(i)]["Hit"];

			if (hit.data_type() == nl::node::type::audio)
				hitsounds.emplace(i, Sound(hit));
		}
	}

	std::vector<int32_t> Reactor::event_types(int8_t s) const
	{
		std::vector<int32_t> types;
		nl::node events = src[std::to_string(s)]["event"];

		for (size_t i = 0; events[std::to_string(i)]; i++)
			types.push_back(events[std::to_string(i)]["type"].get_integer());

		return types;
	}

	std::vector<reactor_rules::SkillEvent> Reactor::skill_events(int8_t s) const
	{
		std::vector<reactor_rules::SkillEvent> found;
		nl::node events = src[std::to_string(s)]["event"];

		for (size_t i = 0; events[std::to_string(i)]; i++)
		{
			nl::node event = events[std::to_string(i)];
			nl::node skills = event["activeSkillID"];
			Point<int16_t> lt = event["lt"];
			Point<int16_t> rb = event["rb"];

			reactor_rules::SkillEvent skill_event;
			skill_event.type = event["type"].get_integer();
			skill_event.area = { lt.x(), lt.y(), rb.x(), rb.y() };

			for (size_t j = 0; skills[std::to_string(j)]; j++)
				skill_event.skills.push_back(skills[std::to_string(j)].get_integer());

			found.push_back(std::move(skill_event));
		}

		return found;
	}

	void Reactor::play_hit(int8_t s)
	{
		auto sound = hitsounds.find(s);

		if (sound != hitsounds.end())
			sound->second.play();
	}

	void Reactor::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		// TODO: Handle 'default' animations (horntail reactor floating)
		animation.draw(absp - Point<int16_t>(0, shift), alpha);
	}

	int8_t Reactor::update(const Physics &physics)
	{
		// A broken reactor is gone; one spawned again comes anew.
		if (!active)
			return -1;

		physics.move_object(phobj);

		cooldown = std::max(cooldown - Constants::TIMESTEP, 0);
		bool ended = animation.update();

		switch (phase)
		{
			case Phase::DELAY:
			case Phase::DESTROY:
				if (deadline > 0)
				{
					deadline -= Constants::TIMESTEP;

					if (deadline <= 0 && !start_hit())
					{
						if (phase == Phase::DESTROY)
							deactivate();

						phase = Phase::IDLE;
					} else if (deadline <= 0 && phase == Phase::DELAY)
					{
						phase = Phase::HIT;
					}
				} else if (phase == Phase::DESTROY && ended)
				{
					deactivate();
				}

				break;
			case Phase::HIT:
				if (ended)
				{
					animation = src[std::to_string(state)];
					phase = Phase::IDLE;
				}

				break;
			default:
				break;
		}

		return phobj.fhlayer;
	}

	bool Reactor::start_hit()
	{
		// Without the server's word, the swing's own flags pick the event,
		// from the state it was in before its last hit. Event -1 from the
		// server is the state's own hit.
		if (event == UNKNOWN_EVENT)
		{
			event = static_cast<int8_t>(reactor_rules::pick_event(event_types(previous), flags));

			if (event < 0)
				return false;
		}

		nl::node from = src[std::to_string(previous)];
		nl::node hit = from["event"][std::to_string(event)]["hit"];

		if (!hit)
			hit = from["hit"];

		animation = hit;
		play_hit(previous);

		return true;
	}

	void Reactor::hit(int8_t s, uint16_t delay, int8_t e, int32_t cool)
	{
		previous = state;
		state = s;

		if (phase == Phase::IDLE)
		{
			phase = Phase::DELAY;
			deadline = std::max<int32_t>(delay, 1);
		}

		event = e;
		cooldown = cool;
	}

	void Reactor::destroy(int8_t s)
	{
		previous = state;
		state = s;
		phase = Phase::DESTROY;
		event = UNKNOWN_EVENT;
		// Breaking plays a hit after 400 ms. Removed in a state it could
		// still leave, it plays none and goes when its animation ends.
		deadline = event_types(state).empty() ? 400 : 0;
	}

	void Reactor::swing(int32_t f, uint16_t delay)
	{
		phase = Phase::DELAY;
		flags = f;
		deadline = std::max<int32_t>(delay, 1);
		event = UNKNOWN_EVENT;
	}

	int8_t Reactor::use_skill(int32_t skill, Point<int16_t> user)
	{
		if (!active || phase != Phase::IDLE || cooldown > 0)
			return -1;

		Point<int16_t> from = user - get_position();
		int32_t index = reactor_rules::pick_skill_event(skill_events(state), skill, from.x(), from.y());

		if (index < 0)
			return -1;

		// The event is this state's, whatever hit the reactor last.
		phase = Phase::DELAY;
		deadline = SKILL_DELAY;
		previous = state;
		event = static_cast<int8_t>(index);

		return event;
	}

	reactor_rules::Box Reactor::get_swing_box() const
	{
		if (!active || phase != Phase::IDLE || cooldown > 0 || !reactor_rules::swingable(event_types(state)))
			return {};

		Point<int16_t> corner = get_position() - Point<int16_t>(0, shift) - animation.get_origin();
		Point<int16_t> size = animation.get_dimensions();

		return { corner.x(), corner.y(), corner.x() + size.x(), corner.y() + size.y() };
	}
}
