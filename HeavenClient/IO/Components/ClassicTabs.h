#pragma once

#include <cstdint>
#include <vector>

#include "../../Graphics/Texture.h"

#include <nlnx/node.hpp>

namespace ms
{
	// A v83 tab strip from Basic.img: tabs of one width side by side, the
	// selected one pink. Neighbouring tabs share a middle piece, so tab i
	// starts at i * pitch(). Each tab's label (enabled when selected) is
	// centred over its fill.
	class ClassicTabs
	{
	public:
		struct Style
		{
			const char* node;
			int16_t edge;
			int16_t height;
			// Labels stand on this line below the strip's top, or hang from
			// it when `label_top` is set.
			int16_t label_y;
			bool label_top;
		};

		// The skill window's strip; its labels stand on the base line.
		static const Style TAB5;
		// The quest log's smaller strip; its labels share a top line, so
		// descenders don't lift them.
		static const Style TAB2;

		ClassicTabs(const Style& style, nl::node enabled, nl::node disabled, int16_t fill);
		ClassicTabs();

		void draw(Point<int16_t> position, uint16_t count, uint16_t selected) const;
		// Only tab index's label, e.g. over a highlight drawn on that tab.
		void draw_label(Point<int16_t> position, uint16_t index, bool selected) const;

		int16_t pitch() const;
		int16_t height() const;

	private:
		Style style = TAB5;
		Texture left[2];
		Texture fill[2];
		// Between two unselected tabs, after the selected one, before it.
		Texture middle[3];
		Texture right[2];
		std::vector<Texture> enabled;
		std::vector<Texture> disabled;
		int16_t fill_width = 0;
	};
}
