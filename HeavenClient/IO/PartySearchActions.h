#pragma once

#include "../Character/PartySearch.h"

#include <cstddef>

namespace ms
{
	// Changes to the party search that send what PartySearch says to, and
	// show the indicator and the window as they now stand (v83's wrappers
	// 0xa12073, 0xa120b9 and 0xa120e7).
	namespace PartySearchActions
	{
		void start(const PartySearch::Wanted& wanted);
		void resume();
		void pause();
		void stop();

		void leaving_channel();
		void field_entered();
		void joined(size_t size);

		// Logging out: forget the search and take the indicator down.
		void reset();
	}
}
