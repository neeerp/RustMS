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

#include <string>

#include <cstdint>

#include "MapObject.h"

#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"
#include "../../IO/Components/ChatBalloon.h"
#include "../../Util/Randomizer.h"
#include "../../Character/QuestMarker.h"
#include <limits>

namespace ms
{
	// Represents a NPC on the current map
	// Implements the 'MapObject' interface to be used in a 'MapObjects' template
	class Npc : public MapObject
	{
	public:
		// Constructs an NPC by combining data from game files with data sent by the server
		Npc(int32_t npcid, int32_t oid, bool mirrored, uint16_t fhid, bool control, Point<int16_t> position);

		// Draws the current animation and name/function tags
		void draw(double viewx, double viewy, float alpha) const override;

		// Updates the current animation and physics.
		int8_t update(const Physics &physics) override;

		// Changes stance and resets animation.
		void set_stance(const std::string &stance);

		// Pick idle actions for everyone, or stop (FUN_006d2024).
		void set_control(bool control);

		// Play an idle action, which the controller chose (FUN_006d2e07):
		// `action` is an index into the actions from 2, or -1 for `line`
		// of the info/speak lines alone; `line` is -1 for none.
		void act(int8_t action, int8_t line);

		// Check whether this is a server-sided NPC
		bool isscripted() const;

		// Check if the NPC is in range of the cursor.
		bool inrange(Point<int16_t> cursorpos, Point<int16_t> viewpos) const;

		// Returns the NPC name
		std::string get_name();

		// Returns the NPC's function description or title.
		std::string get_func();
		int32_t get_npcid() const { return npcid; }

	private:
		Point<int16_t> quest_marker_offset() const;
		int16_t head_height() const;
		// Plays an idle action or says an idle line now and then; see npc_idle.
		void update_idle();
		void wait_idle();

		// Idle lines from info/speak, said in the "npc" balloon style.
		std::vector<std::string> idle_lines;
		ChatBalloon speech;
		// The actions an idle NPC may play: every animation but info, stand
		// and move, in template order, with their speak line counts.
		std::vector<std::string> actions;
		std::vector<int32_t> action_lines;
		int32_t specials = 0;
		int32_t shop = -1;
		// Time until the next idle choice, in milliseconds.
		int32_t idle_cooldown = 0;
		QuestMarker quest_marker = QuestMarker::NONE;
		Animation quest_marker_animation;
		uint64_t quest_revision = std::numeric_limits<uint64_t>::max();
		int16_t quest_job = -1;
		int16_t quest_level = -1;
		int16_t quest_refresh = 0;
		std::map<std::string, Animation> animations;
		std::map<std::string, std::vector<std::string>> lines;
		std::string name;
		std::string func;
		bool hidename;
		bool scripted;
		bool mouseonly;
		// The double-click area around the feet, info/dcLeft..dcBottom (v83
		// 0x6dd58c): (-22,-65) to (22,0) unless given. The Maple TV's is
		// empty, so it doesn't take clicks meant for the NPCs before it.
		Point<int16_t> dc_left_top;
		Point<int16_t> dc_right_bottom;

		int32_t npcid;
		bool flip;
		std::string stance;
		bool control;
		// The controller waits for its last choice to come back.
		bool pending = false;

		Randomizer random;
		Text namelabel;
		Text funclabel;
	};
}
