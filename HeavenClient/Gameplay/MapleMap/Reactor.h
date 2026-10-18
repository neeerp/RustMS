//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include <cstdint>

#include "MapObject.h"
#include "ReactorRules.h"

#include "../Audio/Audio.h"
#include "../Graphics/Animation.h"

#include "../Gameplay/Physics/Physics.h"

#include <vector>
#include <map>

namespace ms
{
	// A reactor plays a hit some time after it is hit, as v83 does
	// (FUN_00734688): the event it plays is the one the server names, or, if
	// the hit is the local player's own and the server hasn't answered yet,
	// one picked here.
	class Reactor : public MapObject
	{
	public:
		Reactor(int32_t oid, int32_t rid, int8_t state, Point<int16_t> position);

		void draw(double viewx, double viewy, float alpha) const override;

		int8_t update(const Physics &physics) override;

		// The server moved it on to `state` by `event` of the state before,
		// to play after `delay` ms, and it takes no swing for `cooldown` ms
		// (0x115).
		void hit(int8_t state, uint16_t delay, int8_t event, int32_t cooldown);

		// The server removed it in `state` (0x118). If that is its last
		// state it plays the hit first.
		void destroy(int8_t state);

		// The local player swung at it with these flags, to hit it after
		// `delay` ms.
		void swing(int32_t flags, uint16_t delay);

		// Where a swing hits it, or an empty box if it takes none now.
		reactor_rules::Box get_swing_box() const;

	private:
		enum class Phase
		{
			IDLE,
			// A hit plays when the deadline passes.
			DELAY,
			HIT,
			// Any hit plays when the deadline passes, then it goes.
			DESTROY
		};

		// An event not yet named by the server.
		static constexpr int8_t UNKNOWN_EVENT = -2;

		int32_t rid;
		nl::node src;

		int8_t state;
		int8_t previous;
		Phase phase;
		int32_t deadline;
		int32_t cooldown;
		int8_t event;
		int32_t flags;

		Animation animation;
		// The reactor is drawn this far up from its foothold.
		int16_t shift;

		// Sound/Reactor.img/<id>/<state>/Hit, played with that state's hit
		// animation.
		std::map<int8_t, Sound> hitsounds;

		std::vector<int32_t> event_types(int8_t state) const;
		bool start_hit();
		void play_hit(int8_t state);
	};
}
