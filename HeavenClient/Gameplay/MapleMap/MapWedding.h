#pragma once

#include "WeddingRules.h"

#include "../../Graphics/Animation.h"
#include "../../Template/Point.h"

#include <optional>

namespace ms
{
	// A wedding's altar: the priest's pages and the hearts over the couple
	// (report b075). Nothing on any other map.
	class MapWedding
	{
	public:
		MapWedding(int32_t mapid);

		MapWedding();

		// WEDDING_PROGRESS: the priest's `step`; `self` is the player.
		void progress(uint8_t step, int32_t groom, int32_t bride, int32_t self);

		// WEDDING_CEREMONY_END: the open page ends and the hearts show.
		void show_hearts();

		// The priest's page `page` was answered: 1 for NEXT or Yes.
		void answered(uint32_t page, int8_t response);

		int32_t get_groom() const;
		int32_t get_bride() const;

		// Where the groom and the bride stand now, if here.
		void update(std::optional<Point<int16_t>> groompos, std::optional<Point<int16_t>> bridepos);

		// The hearts, midway between the couple, while both are here; on
		// the effect layer (z 0xc00614a4).
		void draw(Point<int16_t> viewpos, float alpha) const;

	private:
		// Ends the open page without its answer, if it is still up: whether
		// it was. A page another dialog replaced is forgotten.
		bool close_page();

		// Every page waiting returns.
		void settle();

		int32_t mapid = 0;
		int32_t priest = 0;
		// The latest step (+0x7f0), the couple (+0x7fc, +0x800) and
		// whether the player is one of them.
		uint8_t step = 0;
		int32_t groom = 0;
		int32_t bride = 0;
		bool couple = false;
		wedding::Pages pages;
		// Which page the open dialog is, so an old one's answer is ignored.
		uint32_t page = 0;
		bool hearts = false;
		Animation effect;
		std::optional<Point<int16_t>> hearts_pos;
	};
}
