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
#include "ItemTooltip.h"

#include "../Character/Inventory/ItemExpiry.h"
#include "../Character/Inventory/ItemProperties.h"
#include "../Character/Marriage.h"
#include "../Data/ItemData.h"
#include "../Gameplay/Stage.h"
#include "../Util/NxFiles.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// Measured from period footage.
		constexpr int16_t WIDTH = 290;
		// The orange lines under the name, centred and 19 apart (FUN_008edfc7).
		constexpr int16_t PROPERTY_Y = 25;
		constexpr int16_t PROPERTY_STEP = 19;
		const Point<int16_t> DESC_POS = { 91, 29 };
		constexpr int16_t DESC_WIDTH = 186;
		// The expiry line's bullet, with the text level with it (FUN_008edfc7). The icon and description move
		// down to make room.
		const Point<int16_t> EXPIRY_DOT = { 10, 33 };
		const Point<int16_t> EXPIRY_POS = { 16, 22 };
		constexpr int16_t EXPIRY_HEIGHT = 20;
		// A pet's tooltip has its Water of Life there instead, and makes 16 px of room (FUN_008ed8f5).
		constexpr int16_t PET_LINE_HEIGHT = 16;
		// The sender of a present: a dot line 16 under the date, or a pet's centred line, either making 16 px of
		// room.
		constexpr int16_t GIFT_STEP = 16;
		// A token makes 16 px of room for its partner line, record or not (0x8ee4c3). v83 then starts the expiry
		// line 19 lower; tokens have none.
		constexpr int16_t TOKEN_HEIGHT = 16;
		// Who made it: a centred line under the date, making 16 px of room (0x8ef29c).
		constexpr int16_t MAKER_STEP = 16;

		Text property_line(const std::string &line)
		{
			return line.empty() ? Text() : Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::TIPORANGE, line);
		}
	}

	ItemTooltip::ItemTooltip() :
		expiry_dot(NxFiles::ui_classic["UIWindow.img"]["ToolTip"]["Equip"]["Dot"]["1"])
	{
		itemid = 0;
		expiration = 0;
		flags = 0;
		owned = false;
		remain_life = 0;
		cash_shop = false;
		properties_height = 0;
		marriage_height = 0;
		expiry_height = 0;
	}

	bool ItemTooltip::set_item(int32_t iid, int64_t expire, int16_t flag, bool own, int32_t remain,
							   const std::string &made_by)
	{
		if (!cash_shop && itemid == iid && expiration == expire && flags == flag && owned == own &&
			remain_life == remain && owner == made_by)
			return false;

		itemid = iid;
		expiration = expire;
		flags = flag;
		owned = own;
		remain_life = remain;
		owner = made_by;
		cash_shop = false;

		if (itemid == 0)
			return false;

		const ItemData &idata = ItemData::get(itemid);
		bool pet = itemid / 10000 == 500;
		build(pet ? item_expiry::pet_line(expiration, remain_life, idata.get_pet_life())
				  : item_expiry::line(expiration, idata.is_cashitem(), flags & item_expiry::SEALED), "");

		return true;
	}

	void ItemTooltip::set_commodity(int32_t iid, int32_t period, const std::string &descstr)
	{
		itemid = iid;
		expiration = 0;
		flags = 0;
		owned = false;
		remain_life = 0;
		owner.clear();
		cash_shop = true;

		bool pet = itemid / 10000 == 500;
		build(pet ? item_expiry::commodity_pet_line(ItemData::get(itemid).get_pet_life())
				  : item_expiry::commodity_line(itemid, period), "", descstr);
	}

	void ItemTooltip::set_cash_item(int32_t iid, int64_t expire, const std::string &gift_from)
	{
		itemid = iid;
		expiration = expire;
		flags = 0;
		owned = false;
		remain_life = 0;
		owner.clear();
		cash_shop = true;

		bool pet = itemid / 10000 == 500;
		build(pet ? "" : item_expiry::line(expiration, true, false), gift_from.empty() ? "" : "A GIFT FROM '" + gift_from + "'");
	}

	void ItemTooltip::build(const std::string &expirystr, const std::string &giftstr, const std::string &own_desc)
	{
		const ItemData &idata = ItemData::get(itemid);

		const item_properties::Info &info = idata.get_properties();
		item_properties::Lines lines = item_properties::bundle(info, flags);
		properties = { property_line(lines.first), property_line(lines.second) };
		properties_height = PROPERTY_STEP * ((lines.first.empty() ? 0 : 1) + (lines.second.empty() ? 0 : 1));

		const Player &player = Stage::get().get_player();
		marriage_line = property_line(marriage::tooltip_line(itemid, player.get_marriage(), player.get_stats().get_female()));
		marriage_height = marriage::is_token(itemid) ? TOKEN_HEIGHT : 0;

		// One the player has, and could trade with Scissors of Karma, says so.
		std::string descstr = own_desc.empty() ? idata.get_desc() : own_desc;

		if (owned && info.trade_available && !(flags & item_properties::BUNDLE_KARMA))
			descstr = item_properties::with_karma_hint(descstr);

		itemicon = idata.get_icon(false);
		name = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE, idata.get_name(), WIDTH - 2 * tooltip::NAME_POS.x());
		desc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, descstr, DESC_WIDTH, true, tooltip::LINE_ADJ);

		bool pet = itemid / 10000 == 500;
		expiry = expirystr.empty() ? Text() : Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, expirystr);
		expiry_height = expiry.empty() ? 0 : pet ? PET_LINE_HEIGHT : EXPIRY_HEIGHT;

		// Pets have no line for it (FUN_008ed8f5).
		if (!pet && item_properties::shows_maker(itemid, owner))
		{
			maker_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Specially made by ");
			maker = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::TIPORANGE, owner);
		}
		else
		{
			maker_label = Text();
			maker = Text();
		}

		if (giftstr.empty())
			gift = Text();
		else if (pet)
			gift = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE, giftstr);
		else
			gift = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, giftstr);

		int16_t top = properties_height + marriage_height + expiry_height + (maker.empty() ? 0 : MAKER_STEP) +
					  (gift.empty() ? 0 : GIFT_STEP);
		int16_t bottom = std::max<int16_t>(tooltip::ICON_POS.y() + tooltip::ICON_BOX, DESC_POS.y() + desc.height());
		size = Point<int16_t>(WIDTH, top + bottom + tooltip::BOTTOM);
	}

	void ItemTooltip::draw(Point<int16_t> pos) const
	{
		if (itemid == 0)
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		icon.draw_dot(pos + tooltip::DOT_POS);
		name.draw(pos + tooltip::NAME_POS);

		for (size_t i = 0; i < properties.size(); i++)
			if (!properties[i].empty())
				properties[i].draw(pos + Point<int16_t>(WIDTH / 2, PROPERTY_Y + PROPERTY_STEP * static_cast<int16_t>(i)));

		pos.shift_y(properties_height);

		if (!marriage_line.empty())
			marriage_line.draw(pos + Point<int16_t>(WIDTH / 2, PROPERTY_Y));

		pos.shift_y(marriage_height);

		if (!expiry.empty())
		{
			expiry_dot.draw(pos + EXPIRY_DOT);
			expiry.draw(pos + EXPIRY_POS);
		}

		// The lines under the date, 16 apart: who made it, then who sent it.
		Point<int16_t> line = pos + Point<int16_t>(0, expiry.empty() ? 0 : GIFT_STEP);

		if (!maker.empty())
		{
			// The label and the name are centred as one, level with a dot line's text (FUN_008f49bc mode 0).
			int16_t left = WIDTH / 2 - (maker_label.width() + maker.width()) / 2;
			maker_label.draw(line + Point<int16_t>(left, EXPIRY_POS.y()));
			maker.draw(line + Point<int16_t>(left + maker_label.width(), EXPIRY_POS.y()));

			line.shift_y(MAKER_STEP);
			pos.shift_y(MAKER_STEP);
		}

		if (!gift.empty())
		{
			// FUN_008edfc7 puts it 16 under the date, FUN_008ed8f5 centres a pet's.
			if (itemid / 10000 == 500)
			{
				gift.draw(line + Point<int16_t>(WIDTH / 2, PROPERTY_Y));
			}
			else
			{
				expiry_dot.draw(line + EXPIRY_DOT);
				gift.draw(line + EXPIRY_POS);
			}

			pos.shift_y(GIFT_STEP);
		}

		pos.shift_y(expiry_height);

		icon.draw(itemicon, pos + tooltip::ICON_POS);
		desc.draw(pos + DESC_POS);
	}
}
