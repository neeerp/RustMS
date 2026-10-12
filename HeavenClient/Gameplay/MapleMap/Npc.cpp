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

#include <nlnx/nx.hpp>

namespace ms
{
	Npc::Npc(int32_t id, int32_t o, bool fl, uint16_t f, bool cnt, Point<int16_t> position) : MapObject(o)
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

		phobj.fhid = f;
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
			? -quest_marker_offset().y() + quest_marker_animation.get_dimensions().y() / 2
			: head_height() + 4;
		speech.draw(absp - Point<int16_t>(0, lift));
	}

	int8_t Npc::update(const Physics &physics)
	{
		if (!active)
			return phobj.fhlayer;

		physics.move_object(phobj);
		auto& quests = Stage::get().get_player().get_quests();
		int16_t job = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::JOB);
		int16_t level = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::LEVEL);
		if (quest_revision != quests.revision() || quest_job != job || quest_level != level)
		{
			QuestMarker next = quests.npc_marker(npcid, job, level);
			if (next != quest_marker)
			{
				quest_marker = next;
				if (next != QuestMarker::NONE)
					quest_marker_animation = nl::nx::ui["UIWindow2.img"]["QuestIcon"][std::to_string(static_cast<int>(next))];
			}
			quest_revision = quests.revision();
			quest_job = job;
			quest_level = level;
		}
		if (quest_marker != QuestMarker::NONE) quest_marker_animation.update();

		update_speech();

		if (animations.count(stance))
		{
			bool aniend = animations.at(stance).update();

			if (aniend && states.size() > 0)
			{
				size_t next_stance = random.next_int(states.size());
				std::string new_stance = states[next_stance];
				set_stance(new_stance);
			}
		}

		return phobj.fhlayer;
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
		// The icon's own animation provides its original bobbing motion.
		return Point<int16_t>(0, -head_height() - quest_marker_animation.get_dimensions().y() / 2 - 8);
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
