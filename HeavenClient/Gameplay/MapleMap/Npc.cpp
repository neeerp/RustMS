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
#include "Npc.h"
#include "../Stage.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cstdlib>

namespace ms
{
	namespace
	{
		// The quest icons are 44 px tall, with their origin halfway down.
		constexpr int16_t QUEST_MARKER_HALF = 22;
	}

	Npc::Npc(int32_t id, int32_t o, bool fl, uint16_t f, bool cnt, Point<int16_t> position, int16_t r0, int16_t r1)
		: MapObject(o)
	{
		std::string strid = std::to_string(id);
		strid.insert(0, 7 - strid.size(), '0');
		strid.append(".img");

		nl::node src = nl::nx::npc[strid];
		nl::node strsrc = nl::nx::string["Npc.img"][std::to_string(id)];

		std::string link = src["info"]["link"];

		if (link.size() > 0)
		{
			link.append(".img");
			src = nl::nx::npc[link];
		}

		nl::node info = src["info"];

		hidename = info["hideName"].get_bool();
		mouseonly = info["talkMouseOnly"].get_bool();
		scripted = info["script"].size() > 0 || info["shop"].get_bool();

		for (auto npcnode : src)
		{
			std::string state = npcnode.name();

			if (state != "info")
			{
				animations[state] = npcnode;
				states.push_back(state);
			}

			for (auto speaknode : npcnode["speak"])
				lines[state].push_back(strsrc[speaknode.get_string()]);
		}

		if (lines.count("info"))
			idle_lines = lines["info"];

		speech = ChatBalloon("npc", true);
		// Stagger the first line so the NPCs on a map don't all speak at once.
		speech_cooldown = random.next_int<int32_t>(SPEECH_FIRST_MIN, SPEECH_FIRST_MAX);

		name = std::string(strsrc["name"]);
		func = std::string(strsrc["func"]);

		namelabel = Text(Text::Font::A13B, Text::Alignment::CENTER, Color::Name::YELLOW, Text::Background::NAMETAG,
						 name);
		funclabel = Text(Text::Font::A13B, Text::Alignment::CENTER, Color::Name::YELLOW, Text::Background::NAMETAG,
						 func);

		npcid = id;
		flip = !fl;
		control = cnt;
		stance = "stand";
		rx0 = std::min(r0, r1);
		rx1 = std::max(r0, r1);

		phobj.fhid = f;
		phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);
		set_position(position);
	}

	void Npc::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> absp = phobj.get_absolute(viewx, viewy, alpha);

		if (animations.count(stance))
			animations.at(stance).draw(DrawArgument(absp, flip), alpha);

		if (!hidename)
		{
			// If ever changing code for namelabel confirm placements with map 10000
			namelabel.draw(absp + Point<int16_t>(0, -4));
			funclabel.draw(absp + Point<int16_t>(0, 18));
		}
		if (quest_marker != QuestMarker::NONE)
			quest_marker_animation.draw(DrawArgument(absp + quest_marker_offset()), alpha);

		// The balloon sits above the head, and above the quest marker if there is one.
		int16_t lift = quest_marker != QuestMarker::NONE
			? -quest_marker_offset().y() + QUEST_MARKER_HALF
			: head_height() + 4;
		speech.draw(absp - Point<int16_t>(0, lift));
	}

	int8_t Npc::update(const Physics &physics)
	{
		if (!active)
			return phobj.fhlayer;

		update_walk();
		physics.move_object(phobj);
		auto& player = Stage::get().get_player();
		auto& quests = player.get_quests();
		int16_t job = player.get_stats().get_stat(MapleStat::Id::JOB);
		int16_t level = player.get_stats().get_stat(MapleStat::Id::LEVEL);
		// Item requirements can change without a quest update, so also
		// refresh now and then.
		if (quest_revision != quests.revision() || quest_job != job || quest_level != level || ++quest_refresh >= 30)
		{
			const Inventory& inventory = player.get_inventory();
			QuestMarker next = quests.npc_marker(npcid, job, level,
				[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); });
			if (next != quest_marker)
			{
				quest_marker = next;
				if (next != QuestMarker::NONE)
					quest_marker_animation = NxFiles::ui_classic["UIWindow.img"]["QuestIcon"][std::to_string(static_cast<int>(next))];
			}
			quest_revision = quests.revision();
			quest_job = job;
			quest_level = level;
			quest_refresh = 0;
		}
		if (quest_marker != QuestMarker::NONE) quest_marker_animation.update();

		update_speech();

		if (animations.count(stance))
		{
			bool aniend = animations.at(stance).update();

			// A walk keeps its stance until it ends.
			if (aniend && !walking)
				next_stance();
		}

		return phobj.fhlayer;
	}

	void Npc::next_stance()
	{
		if (states.empty())
			return;

		std::string next = states[random.next_int(states.size())];

		if (next == "move")
		{
			int16_t x = get_position().x();
			int16_t target = random.next_int<int16_t>(rx0, rx1 + 1);

			if (rx1 - rx0 < WALK_MIN || std::abs(target - x) < WALK_MIN)
			{
				next = "stand";
			}
			else
			{
				walking = true;
				walk_target = target;
				flip = target > x;
			}
		}

		set_stance(next);
	}

	void Npc::update_walk()
	{
		if (!walking)
			return;

		int16_t x = get_position().x();
		bool arrived = flip ? x >= walk_target : x <= walk_target;

		// A wall or the edge of the floor ends the walk early.
		if (arrived || phobj.is_flag_not_set(PhysicsObject::Flag::TURNATEDGES))
		{
			walking = false;
			phobj.set_flag(PhysicsObject::Flag::TURNATEDGES);
			set_stance("stand");
			return;
		}

		phobj.hforce = flip ? WALKFORCE : -WALKFORCE;
	}

	void Npc::set_stance(const std::string &st)
	{
		if (stance != st)
		{
			stance = st;

			auto iter = animations.find(stance);

			if (iter == animations.end())
				return;

			iter->second.reset();
		}
	}

	bool Npc::isscripted() const
	{
		return scripted;
	}

	bool Npc::inrange(Point<int16_t> cursorpos, Point<int16_t> viewpos) const
	{
		if (!active)
			return false;

		Point<int16_t> absp = get_position() + viewpos;

		Point<int16_t> dim =
				animations.count(stance) ?
				animations.at(stance).get_dimensions() :
				Point<int16_t>();

		return Rectangle<int16_t>(
				absp.x() - dim.x() / 2,
				absp.x() + dim.x() / 2,
				absp.y() - dim.y(),
				absp.y()
		).contains(cursorpos) || (quest_marker != QuestMarker::NONE &&
			Rectangle<int16_t>(absp + quest_marker_offset() - quest_marker_animation.get_origin(),
				absp + quest_marker_offset() - quest_marker_animation.get_origin() + quest_marker_animation.get_dimensions()).contains(cursorpos));
	}

	int16_t Npc::head_height() const
	{
		// Anchor to the standing sprite so changing NPC poses does not move what
		// is drawn above the head.
		auto stand = animations.find("stand");
		int16_t height = 50;
		if (stand != animations.end())
		{
			height = stand->second.get_origin().y();
			if (height <= 0) height = stand->second.get_dimensions().y();
		}
		return height;
	}

	Point<int16_t> Npc::quest_marker_offset() const
	{
		// As in footage (Robin, Maria), the bubble sits right of centre, its
		// tail resting on the head. The icon's frames differ in height, but
		// each has its origin at the same point of the art, so this is fixed.
		return Point<int16_t>(20, -head_height() - 15);
	}

	void Npc::update_speech()
	{
		speech.update();

		if (idle_lines.empty() || speech.is_active())
			return;

		speech_cooldown -= Constants::TIMESTEP;

		if (speech_cooldown > 0)
			return;

		speech.change_text(idle_lines[random.next_int(idle_lines.size())]);
		speech_cooldown = random.next_int<int32_t>(SPEECH_PAUSE_MIN, SPEECH_PAUSE_MAX);
	}

	std::string Npc::get_name()
	{
		return name;
	}

	std::string Npc::get_func()
	{
		return func;
	}
}
