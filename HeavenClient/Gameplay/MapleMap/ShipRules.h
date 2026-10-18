#pragma once

#include <algorithm>
#include <cstdint>
#include <string>

namespace ms
{
	// The ship a ship field (info/fieldType 2) shows from its shipObj node,
	// and how CONTI_MOVE and CONTI_STATE move it, as v83 does.
	namespace ship
	{
		// shipObj/shipKind: a ship that sails in to dock at a station and
		// out again, or the Crimson Balrog's, which comes alongside a deck.
		constexpr int32_t DOCKING = 0;
		constexpr int32_t BALROG = 1;

		enum class Action
		{
			NONE,
			// Sail in from x0 to the dock at x, whistling (FUN_0054d32c).
			ENTER,
			// Sail out from the dock to x0, whistling (FUN_0054d1c0).
			LEAVE,
			// Fade in and slide alongside (FUN_0054d498).
			APPEAR,
			// Fade out (FUN_0054d800).
			DISAPPEAR
		};

		// CONTI_MOVE (FUN_0054dca3): the map's part in the trip, then what
		// happened. Only the platforms and the deck have a ship to move.
		inline Action on_move(uint8_t target, uint8_t event, int32_t kind)
		{
			switch (target)
			{
				case 8:
					return kind == DOCKING && event == 2 ? Action::LEAVE : Action::NONE;
				case 10:
					if (kind != BALROG)
						return Action::NONE;

					return event == 4 ? Action::APPEAR : event == 5 ? Action::DISAPPEAR : Action::NONE;
				case 12:
					return kind == DOCKING && event == 6 ? Action::ENTER : Action::NONE;
				default:
					return Action::NONE;
			}
		}

		// CONTI_STATE (FUN_0054dd08): where the trip is, which the client
		// asks on entering the map. A docking ship replays its arrival
		// between trips and while boarding, else its departure; the Balrog's
		// comes while it is out on the way.
		inline Action on_state(uint8_t state, uint8_t flag, int32_t kind)
		{
			if (kind == DOCKING)
			{
				switch (state)
				{
					case 0:
					case 1:
					case 6:
						return Action::ENTER;
					case 2:
					case 3:
					case 4:
					case 5:
						return Action::LEAVE;
					default:
						return Action::NONE;
				}
			}

			if (kind == BALROG && (state == 3 || state == 4) && flag == 1)
				return Action::APPEAR;

			return Action::NONE;
		}

		// The Map.nx path of a shipObj's art. The value names it from the
		// root ("Map/Obj/vehicle.img/ship/ossyria/99"), some without the
		// ".img" ("Map/Obj/vehicle/ship/ossyria/97").
		inline std::string obj_path(std::string path)
		{
			if (path.rfind("Map/", 0) == 0)
				path.erase(0, 4);

			if (path.rfind("Obj/", 0) != 0)
				return path;

			size_t end = path.find('/', 4);

			if (end == std::string::npos || end == 4)
				return path;

			if (end < 8 || path.compare(end - 4, 4, ".img") != 0)
				path.insert(end, ".img");

			return path;
		}

		// shipObj/y is a signed short that Map.nx keeps unsigned (65341 for
		// -195).
		inline int32_t signed_y(int64_t stored)
		{
			return static_cast<int16_t>(static_cast<uint16_t>(stored));
		}

		// A value moving from one number to another at an even pace, as a
		// layer's RelMove does.
		struct Glide
		{
			double from = 0.0;
			double to = 0.0;
			int32_t duration = 0;
			int32_t elapsed = 0;

			void jump(double value)
			{
				from = value;
				to = value;
				duration = 0;
				elapsed = 0;
			}

			void start(double start, double end, int32_t ms)
			{
				from = start;
				to = end;
				duration = ms;
				elapsed = 0;
			}

			void update(int32_t ms)
			{
				elapsed = std::min(elapsed + ms, duration);
			}

			double get() const
			{
				if (duration <= 0)
					return to;

				return from + (to - from) * elapsed / duration;
			}
		};

		// The ms between a Balrog ship's hover steps: every 40 field
		// updates of 30 ms (FUN_0054dc2d).
		constexpr int32_t HOVER_INTERVAL = 1200;

		// Which way a hover step goes: back toward the box if outside it,
		// else as drawn.
		inline int32_t hover_sign(int32_t at, int32_t low, int32_t high, bool negative)
		{
			if (at < low)
				return 1;

			if (high < at)
				return -1;

			return negative ? -1 : 1;
		}

		// Where a map's ship is and how solid. Every animation starts over
		// from its own start point, as v83's do.
		class Motion
		{
		public:
			// From shipObj: kind, x, x0, y, f and tMove (seconds).
			Motion(int32_t kind, int32_t x, int32_t x0, int32_t y, bool flip, int32_t tmove)
				: kind(kind), x(x), x0(x0), y(y), flip(flip), tmove(tmove * 1000), left(x - 50), right(x + 50),
				  top(y - 100), bottom(y + 100)
			{
				// A docking ship is drawn from the start, away from the dock;
				// the Balrog's not until it comes.
				shown = kind == DOCKING;
				px.jump(kind == DOCKING ? x0 : x);
				py.jump(y);
				opacity.jump(1.0);
			}

			int32_t get_kind() const
			{
				return kind;
			}

			bool get_flip() const
			{
				return flip;
			}

			// Whether it is drawn at all.
			bool is_shown() const
			{
				return shown;
			}

			double get_x() const
			{
				return px.get();
			}

			double get_y() const
			{
				return py.get();
			}

			double get_opacity() const
			{
				return opacity.get();
			}

			// Each returns whether it happened, to sound it.
			bool enter()
			{
				if (kind != DOCKING)
					return false;

				px.start(x0, x, tmove);
				py.jump(y);
				return true;
			}

			bool leave()
			{
				if (kind != DOCKING)
					return false;

				px.start(x, x0, tmove);
				py.jump(y);
				return true;
			}

			bool appear()
			{
				if (kind != BALROG)
					return false;

				shown = true;
				opacity.start(0.0, 1.0, tmove);
				px.start(x + (flip ? -100 : 100), x, tmove);
				py.start(y - 100, y, tmove);
				return true;
			}

			bool disappear()
			{
				if (kind != BALROG || !shown || opacity.get() <= 0.0)
					return false;

				opacity.start(opacity.get(), 0.0, tmove);
				return true;
			}

			// Advance `ms`. A Balrog ship hovers about, with steps drawn
			// from `next(n)`, a number below n.
			template<class Next>
			void update(int32_t ms, Next next)
			{
				px.update(ms);
				py.update(ms);
				opacity.update(ms);

				if (kind != BALROG)
					return;

				since_hover += ms;

				if (since_hover < HOVER_INTERVAL)
					return;

				since_hover -= HOVER_INTERVAL;

				if (shown)
					hover(next);
			}

		private:
			// FUN_0054d99e: jump to the last step's end, then glide 20 to 49
			// px each way over 1 to 1.9 s. Only the direction keeps it near
			// the box, so it may stray up to 49 px past an edge.
			template<class Next>
			void hover(Next next)
			{
				px.jump(x);
				py.jump(y);

				int32_t dx = next(30) + 20;
				int32_t sx = hover_sign(x, left, right, next(10) < 5);
				int32_t dy = next(30) + 20;
				int32_t sy = hover_sign(y, top, bottom, next(10) < 5);
				int32_t duration = (next(10) + 10) * 100;

				x += sx * dx;
				y += sy * dy;
				px.start(px.get(), x, duration);
				py.start(py.get(), y, duration);
			}

			int32_t kind;
			// Where it docks or hovers; a hover moves it.
			int32_t x;
			int32_t x0;
			int32_t y;
			bool flip;
			// Every animation's ms.
			int32_t tmove;
			// The box a Balrog ship hovers in.
			int32_t left;
			int32_t right;
			int32_t top;
			int32_t bottom;
			bool shown;
			Glide px;
			Glide py;
			Glide opacity;
			int32_t since_hover = 0;
		};
	}
}
