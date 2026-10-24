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

#include "../Template/Point.h"

#include <cstdint>

namespace ms
{
	// Interface for tooltips, information windows about something
	// the mouse cursor is pointed at.
	class Tooltip
	{
	public:
		// Possible parent UIs for Tooltips.
		enum Parent
		{
			NONE,
			EQUIPINVENTORY,
			ITEMINVENTORY,
			SKILLBOOK,
			SKILLMACRO,
			SHOP,
			STORAGE,
			EVENT,
			TEXT,
			KEYCONFIG,
			WORLDMAP,
			MINIMAP,
			MONSTERBOOK,
			BUFFLIST,
			TRADINGROOM,
			PERSONALSHOP,
			ITEMMEGAPHONE,
			VICIOUSHAMMER,
			MAKER,
			VEGASPELL,
			ITEMPROTECTOR,
			KARMASCISSORS,
			VEGARESULT,
			ENCHANTSKILL,
			STATUSBAR,
			// The status bar's quick slots, whose tooltips go above the cursor.
			QUICKSLOT,
			// The map under the cursor, outside every window.
			FIELD,
			// The Cash Shop's list and Cash Inventory, which show items that aren't in an inventory.
			CASHSHOP,
			// Help from String's ToolTipHelp.img, which belongs to the window it was shown for.
			HELP,
			// The item search's results, whose tooltips go right of the cursor.
			PETITEMSEARCH,
			DUEY,
			FAMILY,
			FAMILYTREE,
			// The Owl of Minerva's results, whose tooltips go below and right of the cursor.
			ITEMSEARCHRESULT,
			WEDDINGREGISTRY,
			USERLIST,
			MEDALS,
			TIMEDQUESTS
		};

		virtual ~Tooltip()
		{}

		virtual void draw(Point<int16_t> cursorpos) const = 0;
	};
}