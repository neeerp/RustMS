#pragma once

#include <string>

namespace ms
{
	// v83 hides a hair layer whose slot (Base.nx smap.img) a hat's
	// info/vslot names: hair H2, hairOverHead and backHair H1,
	// backHairBelowCap H6, backHairBelowCapNarrow H5, backHairBelowCapWide H4,
	// hairShade Hs, hairBelowBody Hb. The hat itself is always drawn.
	namespace hat_slots
	{
		// A vslot is a run of two-letter slots, as "CpH1H5".
		inline bool covers(const std::string& vslot, const char* slot)
		{
			for (size_t i = 0; i + 1 < vslot.size(); i += 2)
				if (vslot[i] == slot[0] && vslot[i + 1] == slot[1])
					return true;

			return false;
		}
	}
}
