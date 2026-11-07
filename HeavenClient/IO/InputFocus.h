#pragma once

#include <cstdint>

namespace ms
{
	// An inactive modal may remain in the focus slot after closing or initial
	// UI construction. It must not consume the key that returns control to play.
	template<class Element, class Background>
	void dispatch_focused_key(Element* element, int32_t action, bool pressed,
		bool escape, Background background)
	{
		if (element && element->is_active())
			element->send_key(action, pressed, escape);
		else
			background();
	}

	// With an NPC's page and a notice both up, the keys go to whichever is on
	// top, as the mouse does. A prompt's complaint comes over its page; a page
	// that comes after a notice, as the booking's does, covers it until it closes.
	inline bool npc_page_hears_keys(bool page_open, bool notice_open, bool page_in_front)
	{
		return page_open && (!notice_open || page_in_front);
	}
}
