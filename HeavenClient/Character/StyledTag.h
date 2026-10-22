#pragma once

#include "MedalTagLayout.h"

#include "../Graphics/Text.h"
#include "../Graphics/Texture.h"

#include <nlnx/node.hpp>

namespace ms
{
	// A name tag v83 builds with FUN_005f0334 from a style under UI
	// NameTag.img: the left cap `w`, the tile `c` repeated under the text,
	// the right cap `e`, and the text's colour `clr`. Without the style's
	// art it is the plain translucent box (0x5f0e1d).
	class StyledTag
	{
	public:
		StyledTag(nl::node style, const std::string &text);
		StyledTag();

		bool empty() const;

		// Draws the tag centred on pos.x with its top at pos.y.
		void draw(Point<int16_t> pos) const;

	private:
		bool shown;
		bool art;
		Texture left, tile, right;
		MedalTagLayout::Layout layout;
		Text text;
		Color color;
	};
}
