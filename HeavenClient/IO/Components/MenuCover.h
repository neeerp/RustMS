#pragma once

#include "../../Template/Rectangle.h"

#include <cstdint>

namespace ms
{
	// The status bar's game and shortcut menus open over its icon row and
	// quick slots. Inside an open menu only the menu's own buttons take the
	// cursor; the status bar buttons under it are covered.
	namespace menu_cover
	{
		inline bool covered(bool menu_open, Rectangle<int16_t> menu, bool menu_button, Point<int16_t> cursor)
		{
			return menu_open && !menu_button && menu.contains(cursor);
		}
	}
}
