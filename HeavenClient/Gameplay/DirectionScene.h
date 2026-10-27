#pragma once

#include "DirectionSceneRules.h"

#include "../Graphics/Animation.h"
#include "../Template/Point.h"

#include <cstdint>
#include <optional>
#include <vector>

namespace ms
{
	// A scene's parts as the server sends them (SHOW_ITEM_GAIN_INCHAT mode 0x12), each from its node in Effect.nx
	// ("Direction2.img/open/back0"): its visuals play once in the field, in order of z, and its field event asks for
	// the field the scene leads to. A new field drops what hasn't come due (report b201).
	class DirectionScene
	{
	public:
		// Start a part's events now, those without a position at `position`.
		void play(nl::node part, Point<int16_t> position);

		// One timestep on: the field a field event came due for, if one did.
		std::optional<int32_t> update();

		void draw(Point<int16_t> viewpos, float alpha) const;

		void clear();

	private:
		struct Visual
		{
			direction::Event event;
			Animation animation;
			// When it began, in ms of the scene's clock.
			int32_t began;
		};

		int32_t now = 0;
		std::vector<direction::Event> waiting;
		std::vector<Visual> showing;
	};
}
