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
#include "SkillTooltip.h"

#include "../Data/SkillData.h"

#include "../../Character/FairyBlessing.h"
#include "../../Character/SpResetRules.h"
#include "../../Gameplay/Stage.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// Measured from period footage.
		constexpr int16_t WIDTH = 312;
		const Point<int16_t> DESC_POS = { 87, 29 };
		constexpr int16_t DESC_WIDTH = 215;
		// The line over the levels' effects sits well below the icon.
		const int16_t LINE_MIN_Y = tooltip::ICON_POS.y() + tooltip::ICON_BOX + 49;
		constexpr int16_t LINE_DESC_GAP = 8;
		constexpr int16_t LINE_X = 6;
		const Point<int16_t> LEVEL_POS = { 9, 4 };
		constexpr int16_t LEVEL_WIDTH = 294;
		// FUN_008f25d0 sizes the box 0x80 tall plus any description lines
		// past the fourth.
		constexpr int16_t NO_LEVELS_HEIGHT = 0x80;
	}

	SkillTooltip::SkillTooltip() :
		line(WIDTH - 2 * LINE_X, Color::Name::WHITE, 1.0f)
	{
		skill_id = 0;
		skill_level = 0;
		skill_masterlevel = 0;
		skill_levels = true;
		line_y = 0;
	}

	void SkillTooltip::set_skill(int32_t id, int32_t level, int32_t mlevel, int64_t expiration, bool levels)
	{
		if (skill_id == id && skill_level == level && skill_masterlevel == mlevel && skill_levels == levels)
			return;

		skill_id = id;
		skill_level = level;
		skill_masterlevel = mlevel;
		skill_levels = levels;

		if (skill_id == 0)
			return;

		const SkillData &data = SkillData::get(id);

		// Skills with a master level go up to the character's own, 0 until one
		// is taught; the rest to their data's (FUN_00a08e05).
		bool own_master = sp_reset::has_master_level(id);
		int32_t masterlevel = own_master ? mlevel : data.get_masterlevel();

		std::string descstr = data.get_desc();

		// Blessing of the Fairy, once it has a level, names the linked
		// character instead (0x8f29c8).
		if (fairy_blessing::is_blessing(id) && level > 0)
			descstr = fairy_blessing::description(Stage::get().get_player().get_linked_name());

		// Other skills' descriptions carry their "[Master Level : N]" already;
		// these get a line of their own above it (0xf0c, 0x8f2909).
		if (own_master)
			descstr.insert(0, "[Master Level : " + std::to_string(mlevel) + "]\\n");

		const std::string exptag = "#cAvailable until";

		if (expiration > 0)
		{
			// TODO: Blank
		} else
		{
			size_t expstart = descstr.find(exptag);
			size_t expend = descstr.find('#', expstart + 1);

			if (expstart < expend && expend != std::string::npos)
			{
				size_t expsize = expend - expstart + 1;
				descstr.erase(expstart, expsize);
			}
		}

		std::string levelstr;
		bool current = levels && level > 0;
		bool next = levels && level < masterlevel;

		if (current)
			levelstr += "[Current Level " + std::to_string(level) + "]\\n" + data.get_level_desc(level);

		if (current && next)
			levelstr += "\\n";

		if (next)
			levelstr += "[Next Level " + std::to_string(level + 1) + "]\\n" + data.get_level_desc(level + 1);

		skillicon = data.get_icon(SkillData::Icon::NORMAL);
		name = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE, data.get_name(), WIDTH - 2 * tooltip::NAME_POS.x());
		desc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, descstr, DESC_WIDTH, true, tooltip::LINE_ADJ);
		leveldesc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, levelstr, LEVEL_WIDTH, true, tooltip::LINE_ADJ);
		desc.set_tooltip();
		leveldesc.set_tooltip();

		line_y = std::max<int16_t>(LINE_MIN_Y, DESC_POS.y() + desc.height() + LINE_DESC_GAP);

		if (levels)
			size = Point<int16_t>(WIDTH, line_y + LEVEL_POS.y() + leveldesc.height() + tooltip::BOTTOM);
		else
			size = Point<int16_t>(WIDTH, std::max<int16_t>(NO_LEVELS_HEIGHT, DESC_POS.y() + desc.height() + tooltip::BOTTOM));
	}

	void SkillTooltip::draw(Point<int16_t> pos) const
	{
		if (skill_id == 0)
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		icon.draw_dot(pos + tooltip::DOT_POS);
		name.draw(pos + tooltip::NAME_POS);
		icon.draw(skillicon, pos + tooltip::ICON_POS);
		desc.draw(pos + DESC_POS);

		if (!skill_levels)
			return;

		pos.shift_y(line_y);
		line.draw(pos + Point<int16_t>(LINE_X, 0));
		leveldesc.draw(pos + LEVEL_POS);
	}
}
