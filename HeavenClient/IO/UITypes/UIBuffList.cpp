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
#include "../Data/SkillData.h"
#include "../../Character/MorphRules.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/AttackAndSkillPackets.h"
#include "../Messages.h"
#include "../UI.h"
#include "../Util/NxFiles.h"

#include <nlnx/node.hpp>

namespace ms
{
	namespace
	{
		// FUN_007b3176: a skill's icon, or an item's iconRaw (icon for
		// 238xxxx items).
		Texture buff_icon(int32_t buffid)
		{
			if (buffid >= 0)
				return SkillData::get(buffid).get_icon(SkillData::Icon::NORMAL);

			int32_t itemid = -buffid;

			return ItemData::get(itemid).get_icon(itemid / 10000 != 238);
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

	Cursor::State UIBuffList::send_cursor(bool, Point<int16_t> cursorpos)
	{
		// FUN_007b2e58
		if (auto entry = entry_at(cursorpos))
		{
			if (entry->id >= 0)
				UI::get().show_skill(Tooltip::Parent::BUFFLIST, entry->id, 0, 0, 0, false);
			else
				UI::get().show_item(Tooltip::Parent::BUFFLIST, -entry->id);
		} else
		{
			UI::get().clear_tooltip(Tooltip::Parent::BUFFLIST);
		}

		return Cursor::State::IDLE;
	}

	bool UIBuffList::is_in_range(Point<int16_t> cursorpos) const
	{
		return entry_at(cursorpos) != nullptr;
	}

	void UIBuffList::remove_cursor()
	{
		UI::get().clear_tooltip(Tooltip::Parent::BUFFLIST);
	}

	void UIBuffList::rightclick(Point<int16_t> cursorpos)
	{
		// FUN_0045b9fc. The icon goes when the server cancels the buff.
		// Dragon Roar's can't be cancelled. An item buff (negative id) sends
		// CANCEL_ITEM_EFFECT, but a morph item's with noCancelMouse is
		// refused while morphed (FUN_00a096af).
		constexpr int32_t DRAGON_ROAR = 1311006;

		if (auto entry = entry_at(cursorpos))
		{
			const Player &player = Stage::get().get_player();

			if (entry->id < 0 && !morph::may_cancel(entry->id, player.get_look().get_morph() > 0,
				player.no_cancel_morph()))
				InChatMessage(Messages::Type::MORPH_FIXED).drop();
			else if (entry->id < 0)
				CancelItemEffectPacket(entry->id).dispatch();
			else if (entry->id != DRAGON_ROAR)
				CancelBuffPacket(entry->id).dispatch();
		}
	}

	UIElement::Type UIBuffList::get_type() const
	{
		return TYPE;
	}

	void UIBuffList::add_buff(int32_t buffid, int32_t stat, int32_t duration)
	{
		row.add(buffid, stat, duration, [buffid] { return buff_icon(buffid); });
		remove_cursor();
	}

	void UIBuffList::cancel_buff(int32_t stat)
	{
		size_t count = row.icons().size();
		row.cancel(stat);

		// The icon under the cursor may have moved or gone; its tooltip comes
		// back when the cursor moves.
		if (row.icons().size() != count)
			remove_cursor();
	}

	const BuffRow<Texture>::Entry* UIBuffList::entry_at(Point<int16_t> cursorpos) const
	{
		auto& icons = row.icons();
		int16_t right = position.x() + CELL;
		int16_t x = cursorpos.x() - (right - CELL * static_cast<int16_t>(icons.size()));
		int16_t y = cursorpos.y() - position.y();

		if (y < 0 || y >= CELL || x < 0 || cursorpos.x() >= right)
			return nullptr;

		return &icons[x / CELL];
	}
}
