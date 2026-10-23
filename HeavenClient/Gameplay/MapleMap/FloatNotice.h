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

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <string>

namespace ms
{
	// One of the status bar's float notices: a line of white text on UIWindow.img's FloatNotice/<type> art, 160 px
	// down the screen (FUN_0058c53b). A weather's message uses one, OX_QUIZ the other.
	class FloatNotice
	{
	public:
		// Puts the text up on the art of the float type. A type with no art leaves the notice as it was and
		// returns false.
		bool show(const std::string &msg, int32_t float_type);
		void clear();
		bool shown() const;

		void draw() const;

	private:
		Texture left;
		Texture tile;
		Texture right;
		int16_t y = 0;
		Text text;
		bool active = false;
	};
}
