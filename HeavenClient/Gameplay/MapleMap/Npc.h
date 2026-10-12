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
		void update_speech();

		// Pauses between idle lines, in milliseconds. The v83 client's exact
		// timing is unknown; these keep NPCs chatty without crowding the screen.
		static constexpr int32_t SPEECH_FIRST_MIN = 2000;
		static constexpr int32_t SPEECH_FIRST_MAX = 10000;
		static constexpr int32_t SPEECH_PAUSE_MIN = 8000;
		static constexpr int32_t SPEECH_PAUSE_MAX = 16000;

		// Idle lines from info/speak, said now and then in the "npc" balloon style.
		std::vector<std::string> idle_lines;
		ChatBalloon speech;
		int32_t speech_cooldown = 0;
		QuestMarker quest_marker = QuestMarker::NONE;
		Animation quest_marker_animation;
		uint64_t quest_revision = std::numeric_limits<uint64_t>::max();
		int16_t quest_job = -1;
		int16_t quest_level = -1;
		int16_t quest_refresh = 0;
		std::map<std::string, Animation> animations;
		std::map<std::string, std::vector<std::string>> lines;
		std::vector<std::string> states;
		std::string name;
		std::string func;
		bool hidename;
		bool scripted;
		bool mouseonly;

		int32_t npcid;
		bool flip;
		std::string stance;
		bool control;

		Randomizer random;
		Text namelabel;
		Text funclabel;
	};
}
