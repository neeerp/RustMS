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

namespace ms
{
	// The v83 login screens are one map, UIClassic.nx MapLogin.img, with a
	// stop for each step stacked 600 pixels apart going up. The windows sit
	// on top and move the camera between stops, under the book frame that
	// borders every step.
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

		// The first step shown cuts straight to its stop; later ones scroll.
		// A step with a new sign slides it in from the left edge.
		void move_to(Step step);

		void draw(float alpha) const;
		void update();

	private:
		bool loaded = false;
		bool visible = false;

		MapBackgrounds backgrounds;
		MapTilesObjs tilesobjs;
		Texture frame;
		Text version;

		double y = 0.0;
		double last_y = 0.0;
		double target_y = 0.0;

		Texture sign;
		uint8_t sign_id = 0;
		double sign_x = 0.0;
		double last_sign_x = 0.0;
	};
}
