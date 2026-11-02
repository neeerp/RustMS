#pragma once

#include "../../Graphics/Animation.h"
#include "../../Template/Point.h"

#include <nlnx/node.hpp>

#include <cstdint>

namespace ms
{
	// The screens of a Maple TV NPC (Npc.nx info MapleTV, e.g. Orbis's
	// 9250046). v83's NPC template loader FUN_006dce02 reads MapleTV and the
	// anchors MapleTVmsgX/Y and MapleTVadX/Y (+0x48 to +0x58), which are
	// offsets from the NPC's foot, never flipped; the NPC's update
	// FUN_006d2918 runs the TV while MapleTV is set. Its layers are above
	// the NPC's (report b-p41-maple-tv-npc). Only the idle TV so far: an ad
	// looping below, the body off above.
	class MapleTvScreen
	{
	public:
		// The anchors as the NPC's info gives them: the NX files store the
		// negative msgX of most TVs as an unsigned short (65516 for -20).
		struct Anchors
		{
			Point<int16_t> message;
			Point<int16_t> ad;
		};

		static bool enabled(nl::node info);
		static Anchors anchors(nl::node info);

		// Where the art goes from the NPC's foot: the body (240x90, origin
		// (0,89)) at (msgX, msgY + 0x5a), the ad (240x180, origin (0,180)) at
		// (adX, adY + 0xb4), report c056 1.2.
		static Point<int16_t> body_position(Anchors anchors);
		static Point<int16_t> ad_position(Anchors anchors);

		MapleTvScreen();
		MapleTvScreen(nl::node info, int32_t ad);

		void update();
		void draw(Point<int16_t> foot, float alpha) const;

	private:
		bool on;
		Anchors at;
		Animation ad;
		Animation body;
	};
}
