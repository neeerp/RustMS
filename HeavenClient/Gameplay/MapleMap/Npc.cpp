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
#include "NpcIdle.h"
#include "../Stage.h"
#include "../../Constants.h"
#include "../../Net/Packets/NpcInteractionPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The quest icons are 44 px tall, with their origin halfway down.
		constexpr int16_t QUEST_MARKER_HALF = 22;
	}

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
				animations[state] = npcnode;

			for (auto speaknode : npcnode["speak"])
				lines[state].push_back(strsrc[speaknode.get_string()]);

			if (state == "info" || state == "stand" || state == "move")
				continue;

			if (state == "shop")
				shop = static_cast<int32_t>(actions.size());

			if (npcnode["special"].get_integer() != 0)
				specials++;

			actions.push_back(state);
			action_lines.push_back(static_cast<int32_t>(lines[state].size()));
		}

		if (lines.count("info"))
			idle_lines = lines["info"];

		speech = ChatBalloon("npc", true);

		name = std::string(strsrc["name"]);
		func = std::string(strsrc["func"]);

		namelabel = Text(Text::Font::A13B, Text::Alignment::CENTER, Color::Name::YELLOW, Text::Background::NAMETAG,
						 name);
		funclabel = Text(Text::Font::A13B, Text::Alignment::CENTER, Color::Name::YELLOW, Text::Background::NAMETAG,
						 func);

		npcid = id;
		flip = !fl;
		stance = "stand";
		set_control(cnt);

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
			? -quest_marker_offset().y() + QUEST_MARKER_HALF
			: head_height() + 4;
		speech.draw(absp - Point<int16_t>(0, lift));
	}

	int8_t Npc::update(const Physics &physics)
	{
		if (!active)
			return phobj.fhlayer;

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
				[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); },
				[&](const QuestInfo& quest) { return player.quest_holdings_met(quest); });
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

		speech.update();
		update_idle();

		if (animations.count(stance))
		{
			bool aniend = animations.at(stance).update();

			// An idle action plays once.
			if (aniend && stance != "stand")
				set_stance("stand");
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
		// As in footage (Robin, Maria), the bubble sits right of centre, its
		// tail resting on the head. The icon's frames differ in height, but
		// each has its origin at the same point of the art, so this is fixed.
		return Point<int16_t>(20, -head_height() - 15);
	}

	void Npc::set_control(bool cnt)
	{
		control = cnt;

		// The first choice also waits.
		if (control)
		{
			wait_idle();
			pending = false;
		}
	}

	void Npc::act(int8_t action, int8_t line)
	{
		pending = false;

		if (action == -1)
		{
			if (line >= 0 && line < static_cast<int32_t>(idle_lines.size()))
				speech.change_text(idle_lines[line]);

			return;
		}

		int32_t index = action - 2;

		if (index < 0 || index >= static_cast<int32_t>(actions.size()))
			return;

		const std::string &name = actions[index];
		set_stance(name);

		if (line >= 0 && line < static_cast<int32_t>(lines[name].size()))
			speech.change_text(lines[name][line]);
	}

	void Npc::update_idle()
	{
		// Only the controller chooses, and everyone plays what comes back.
		if (!control)
			return;

		if (idle_cooldown > 0)
			idle_cooldown -= Constants::TIMESTEP;

		// An action plays out before the next choice.
		if (pending || idle_cooldown > 0 || stance != "stand")
			return;

		wait_idle();

		npc_idle::Choice choice;
		int32_t roll = random.next_int(npc_idle::ROLL);
		int32_t line_roll = random.next_int(npc_idle::ROLL);

		if (!npc_idle::choose(action_lines, specials, shop, static_cast<int32_t>(idle_lines.size()), roll, line_roll,
			choice))
			return;

		int8_t action = choice.action < 0 ? -1 : static_cast<int8_t>(choice.action + 2);
		NpcActionPacket(oid, action, static_cast<int8_t>(choice.line)).dispatch();
		pending = true;
	}

	void Npc::wait_idle()
	{
		idle_cooldown = npc_idle::PAUSE_MIN + random.next_int(npc_idle::PAUSE_SPREAD);
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
