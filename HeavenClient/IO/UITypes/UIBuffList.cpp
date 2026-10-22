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
#include "../../Character/MountRules.h"
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
			if (entry->id == FAMILY)
				show_family();
			else if (entry->id >= 0)
				UI::get().show_skill(Tooltip::Parent::BUFFLIST, entry->id, 0, 0, 0, false);
			else
				UI::get().show_item(Tooltip::Parent::BUFFLIST, -entry->id);
		} else
		{
			remove_cursor();
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
		UI::get().clear_help(TYPE);
	}

	void UIBuffList::rightclick(Point<int16_t> cursorpos)
	{
		// FUN_0045b9fc. The icon goes when the server cancels the buff.
		// Dragon Roar's can't be cancelled, nor Oak Barrel's while it drops
		// or breaks (FUN_0095009a). An item buff (negative id) sends
		// CANCEL_ITEM_EFFECT, but a morph item's with noCancelMouse is
		// refused while morphed (FUN_00a096af). A family entitlement's
		// (type 4) does nothing.
		constexpr int32_t DRAGON_ROAR = 1311006;

		if (auto entry = entry_at(cursorpos); entry && entry->id != FAMILY)
		{
			const Player &player = Stage::get().get_player();

			if (entry->id < 0 && !morph::may_cancel(entry->id, player.get_look().get_morph() > 0,
				player.no_cancel_morph()))
				InChatMessage(Messages::Type::MORPH_FIXED).drop();
			else if (entry->id < 0)
				CancelItemEffectPacket(entry->id).dispatch();
			else if (entry->id != DRAGON_ROAR && !(entry->id == morph::OAK_BARREL && player.is_barrel_busy()))
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

		// The Battleship's icon shows the ship's HP, full as it comes (decomp.c:1324230).
		if (buffid == mount::BATTLESHIP)
		{
			Player &player = Stage::get().get_player();
			int32_t level = player.get_stats().get_stat(MapleStat::Id::LEVEL);
			int32_t max = mount::battleship_max_hp(level, player.get_skills().get_level(buffid));

			if (BuffTimer *timer = row.timer(buffid))
				*timer = BuffTimer::gauge(max);
		}
	}

	void UIBuffList::set_gauge(int32_t buffid, int32_t value)
	{
		if (value == 0)
		{
			row.remove(buffid);
			remove_cursor();
		} else if (BuffTimer *timer = row.timer(buffid))
		{
			timer->set(value);
		}
	}

	void UIBuffList::set_family(uint8_t icon, int32_t left)
	{
		row.remove(FAMILY);

		if (icon != 0)
		{
			row.add(FAMILY, FAMILY, left, [icon] {
				return Texture(NxFiles::ui_classic["UIWindow.img"]["Family"]["RightIcon"][std::to_string(icon)]);
			});
		}

		remove_cursor();
	}

	// The entitlement's name and description, in the box button help uses
	// (0x7b2f02: 0x8e7150 with a7 = 0, a8 = 0, a9 = 1).
	void UIBuffList::show_family() const
	{
		const Family& family = Stage::get().get_player().get_family();
		const std::vector<FamilyPrivilege>& privileges = family.get_privileges();
		int32_t index = family.in_use();

		if (index < 0 || index >= static_cast<int32_t>(privileges.size()))
		{
			UI::get().clear_help(TYPE);
			return;
		}

		UI::get().show_help(TYPE, privileges[index].name, privileges[index].description);
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
