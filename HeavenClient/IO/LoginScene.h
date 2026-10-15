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

#include "../Gameplay/MapleMap/MapBackgrounds.h"
#include "../Gameplay/MapleMap/MapTilesObjs.h"
#include "../Graphics/Text.h"
#include "../Template/Singleton.h"

#include <optional>
#include <vector>

namespace ms
{
	// The v83 login screens are one map, UIClassic.nx MapLogin.img, with a
	// stop for each step stacked 600 pixels apart going up. The windows sit
	// on top and move the camera between stops, under the book frame that
	// borders every step.
	//
	// As in v83 CLogin::ChangeStep, a step change flies the camera to the
	// new stop and fades out the old step's sign. The new step's windows and
	// sign only appear once the camera arrives.
	class LoginScene : public Singleton<LoginScene>
	{
	public:
		enum Step : uint8_t
		{
			TITLE,
			WORLD_SELECT,
			CHAR_SELECT,
			RACE_SELECT,
			CYGNUS_CREATION,
			EXPLORER_CREATION,
			ARAN_CREATION
		};

		void load();
		void unload();

		// The first step shown cuts straight to its stop; later ones fly.
		void move_to(Step step);

		// Whether the camera is still flying to the current step's stop.
		// The step's windows are hidden until it arrives.
		bool moving() const;

		// The names written on the world box from the character select on.
		void set_world(const std::string& world, const std::string& channel);

		void draw(float alpha) const;

		// The world box goes over the step windows, the Cygnus Knights on
		// the race select among them.
		void draw_world_box(float alpha) const;

		void update();

	private:
		// A board that slides in from the left edge once its step arrives,
		// gets a shadow under it, and fades out in place when closed.
		struct Panel
		{
			Texture art;
			int16_t y = 0;
			double slide_time = 0.0;
			double opened = 0.0;
			double closed = -1.0;

			int16_t x(double time) const;
			float opacity(double time) const;
			float shadow_opacity(double time) const;
			bool gone(double time) const;
		};

		void arrive();
		void open_sign(uint8_t id);
		double draw_time(float alpha) const;
		void draw_panel(const Panel& panel, double time) const;

		bool loaded = false;
		bool visible = false;

		MapBackgrounds backgrounds;
		MapTilesObjs tilesobjs;
		Texture frame;
		Texture shadow;
		Text version;

		// Milliseconds since the scene loaded, advanced each update.
		double now = 0.0;

		Step step = Step::TITLE;
		double from_y = 0.0;
		double to_y = 0.0;
		double fly_start = 0.0;
		double fly_end = 0.0;
		bool flying = false;

		uint8_t sign_id = 0;
		std::optional<Panel> sign;
		std::optional<Panel> world_box;
		std::vector<Panel> closing;

		Text world_name;
		Text channel_name;
	};
}
