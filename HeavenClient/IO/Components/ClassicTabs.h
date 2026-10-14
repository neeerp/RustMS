#pragma once

#include <cstdint>
#include <vector>

#include "../../Graphics/Texture.h"

#include <nlnx/node.hpp>

namespace ms
{
	// The v83 tab strip, Basic.img/Tab5: tabs of one width side by side, the
	// selected one pink. Neighbouring tabs share a middle piece, so tab i
	// starts at i * pitch(). Each tab's label (enabled when selected) is
	// centred over its fill and stands on the strip's base line. Measured
	// from period footage of the skill window.
	class ClassicTabs
	{
	public:
		ClassicTabs(nl::node enabled, nl::node disabled, int16_t fill);
		ClassicTabs();

		void draw(Point<int16_t> position, uint16_t count, uint16_t selected) const;
		// Only tab index's label, e.g. over a highlight drawn on that tab.
		void draw_label(Point<int16_t> position, uint16_t index, bool selected) const;

		int16_t pitch() const;

		static constexpr int16_t HEIGHT = 21;

	private:
		static constexpr int16_t EDGE = 6;
		// Labels stand this far below the strip's top.
		static constexpr int16_t LABEL_BOTTOM = 17;

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
