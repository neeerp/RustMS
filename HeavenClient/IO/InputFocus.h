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
}
