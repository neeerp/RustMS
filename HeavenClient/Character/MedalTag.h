#pragma once

#include "MedalTagLayout.h"

#include "../Graphics/Text.h"
#include "../Graphics/Texture.h"

namespace ms
{
	// The tag v83 hangs under a character's name and guild tags for the
	// medal they wear (FUN_00942dcc), in the style of the medal's
	// info/medalTag.
	class MedalTag
	{
	public:
		// The tag for the medal `itemid`: none for 0 or a medal without a
		// name, and a plain name tag when its style has no art.
		MedalTag(int32_t itemid);
		MedalTag();

		int32_t get_itemid() const;

		// Draws the tag centred on pos.x with its top at pos.y.
		void draw(Point<int16_t> pos) const;

	private:
		int32_t itemid;
		bool shown;
		bool art;
		Texture left, tile, right;
		MedalTagLayout::Layout layout;
		Text text;
		Color color;
	};
}
