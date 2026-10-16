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
#include "UIBuffList.h"

#include "../Data/ItemData.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	namespace
	{
		// FUN_007b3176 uses iconRaw, except for skills of job 238.
		Texture buff_icon(int32_t buffid)
		{
			if (buffid < 0)
				return ItemData::get(-buffid).get_icon(true);

			std::string strid = string_format::extend_id(buffid, 7);
			nl::node src = nl::nx::skill[strid.substr(0, 3) + ".img"]["skill"][strid];
			nl::node raw = src["iconRaw"];

			return buffid / 10000 != 238 && raw ? raw : src["icon"];
		}

		// Centred in a cell, whatever the bitmap's origin.
		Point<int16_t> centred(const Texture& texture, int16_t cell)
		{
			Point<int16_t> dimensions = texture.get_dimensions();
			Point<int16_t> corner((cell - dimensions.x()) / 2, (cell - dimensions.y()) / 2);

			return corner + texture.get_origin();
		}
	}

	UIBuffList::UIBuffList()
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"];
		base = src["IconBase"]["0"];

		for (int32_t i = 0; i < BuffTimer::SHADOWS; i++)
			shadows[i] = src["Skill"]["CoolTime"][std::to_string(i)];

		int16_t height = Constants::Constants::get().get_viewheight();
		int16_t width = Constants::Constants::get().get_viewwidth();

		update_screen(width, height);
	}

	void UIBuffList::draw(float) const
	{
		auto& icons = row.icons();
		auto count = static_cast<int16_t>(icons.size());
		Point<int16_t> cell = position - Point<int16_t>(CELL * (count - 1), 0);

		for (auto& entry : icons)
		{
			float opacity = entry.timer.opacity();
			const Texture& shadow = shadows[entry.timer.shadow()];

			base.draw(DrawArgument(cell + centred(base, CELL), opacity));
			entry.icon.draw(DrawArgument(cell + centred(entry.icon, CELL), opacity));
			shadow.draw(DrawArgument(cell + centred(shadow, CELL), opacity));

			cell.shift_x(CELL);
		}
	}

	void UIBuffList::update()
	{
		row.update(Constants::TIMESTEP);
	}

	void UIBuffList::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>(new_width - 35, 23);
		dimension = Point<int16_t>(position.x(), CELL);
	}

	Cursor::State UIBuffList::send_cursor(bool pressed, Point<int16_t> cursorposition)
	{
		return UIElement::send_cursor(pressed, cursorposition);
	}

	UIElement::Type UIBuffList::get_type() const
	{
		return TYPE;
	}

	void UIBuffList::add_buff(int32_t buffid, int32_t stat, int32_t duration)
	{
		row.add(buffid, stat, duration, [buffid] { return buff_icon(buffid); });
	}

	void UIBuffList::cancel_buff(int32_t stat)
	{
		row.cancel(stat);
	}
}
