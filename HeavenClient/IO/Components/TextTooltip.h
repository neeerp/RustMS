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

#include "Tooltip.h"
#include "TooltipBox.h"

#include "../Graphics/Text.h"

namespace ms
{
	// A line or two of text in the v83 tooltip box, as for the map names.
	class TextTooltip : public Tooltip
	{
	public:
		TextTooltip();

		void draw(Point<int16_t> position) const override;

		bool set_text(std::string text);

	private:
		TooltipBox box;
		std::string text;
		Text text_label;
		Point<int16_t> size;
	};
}
