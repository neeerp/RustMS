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

#include <cstdint>

#include "Tooltip.h"
#include "TooltipBox.h"

#include "../Graphics/Geometry.h"
#include "../Graphics/Text.h"

namespace ms
{
	// The v83 skill tooltip (FUN_008f25d0): the name, the icon with the
	// description beside it, and under a line the current and next levels'
	// effects. The buff row's tooltip leaves the levels out.
	class SkillTooltip : public Tooltip
	{
	public:
		SkillTooltip();

		void draw(Point<int16_t> position) const override;

		void set_skill(int32_t id, int32_t level, int32_t masterlevel, int64_t expiration, bool levels);

	private:
		int32_t skill_id;
		int32_t skill_level;
		int32_t skill_masterlevel;
		bool skill_levels;
		Point<int16_t> size;
		int16_t line_y;

		TooltipBox box;
		TooltipIcon icon;
		ColorLine line;

		Texture skillicon;
		Text name;
		Text desc;
		Text leveldesc;
	};
}
