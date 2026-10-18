#pragma once

#include "Movement.h"

#include "../Template/Point.h"

#include <algorithm>
#include <cstdint>
#include <deque>
#include <optional>
#include <vector>

namespace ms
{
	// How the MapleRoyals client batches the user's movement into paths
	// (FUN_0068ab85, FUN_0068a88d) and plays other characters' paths back
	// (FUN_0068b371, FUN_0068adcc).
	namespace move_path
	{
		// A path goes out once it holds this much movement (FUN_0068a828).
		constexpr int32_t SEND_MS = 1000;

		// Elements on a foothold. The rest are in the air (0) or on a ladder
		// or rope (minus its number).
		inline bool grounded(const Movement &element)
		{
			return element.fh > 0;
		}

		struct Path
		{
			Point<int16_t> start;
			std::vector<Movement> elements;
			// Whether it ends as the last path did, with no type 10 element:
			// the user sends no such path.
			bool idle;
		};

		// The user's movement since the last path went out.
		class Recorder
		{
		public:
			// Start over where the user stands, as on entering a map
			// (FUN_0068aab2).
			void reset(const Movement &now)
			{
				elements.clear();
				recorded = 0;
				start = { now.xpos, now.ypos };
				last = now;
			}

			// One step of `ms` milliseconds that ended in `now`. A step that
			// goes on the same way as the last one only makes that one
			// longer.
			void step(Movement now, int16_t ms)
			{
				recorded += ms;

				if (!elements.empty() && extends(elements.back(), now))
				{
					now.duration = elements.back().duration + ms;
					elements.back() = now;
				} else
				{
					now.duration = ms;
					append(now);
				}
			}

			// An element of its own, such as a teleport (4), which takes no
			// time.
			void add(Movement element)
			{
				element.duration = 0;
				append(element);
			}

			// Whether a path should go out: a second of movement with
			// something on a foothold in it, or with anything for a flier
			// (FUN_0068a828).
			bool ready(bool flying = false) const
			{
				return !elements.empty() && recorded >= SEND_MS &&
					   (flying || std::any_of(elements.begin(), elements.end(), grounded));
			}

			// The path to send. What comes after the last element on a
			// foothold waits for the next path, except for a flier, or when
			// nothing is on one.
			Path flush(bool flying = false)
			{
				auto held = elements.end();

				if (!flying)
				{
					held = std::find_if(elements.rbegin(), elements.rend(), grounded).base();

					if (held == elements.begin())
						held = elements.end();
				}

				Path path{ start, std::vector<Movement>(elements.begin(), held), false };
				elements.erase(elements.begin(), held);

				recorded = 0;

				for (const Movement &element : elements)
					recorded += element.duration;

				const Movement &tail = path.elements.back();
				path.idle = tail.command == 0 && !last.hasmoved(tail) && tail.fh == last.fh &&
							std::none_of(path.elements.begin(), path.elements.end(), [](const Movement &element)
							{
								return element.command == 10;
							});

				last = tail;
				start = { tail.xpos, tail.ypos };

				return path;
			}

		private:
			static int sign(int16_t value)
			{
				return (value > 0) - (value < 0);
			}

			static bool extends(const Movement &tail, const Movement &now)
			{
				bool walks = now.command == 0 || now.command == 15 || now.command == 17;

				return walks && tail.command == now.command && tail.fh == now.fh && tail.newstate == now.newstate &&
					   sign(tail.vx) == sign(now.vx) && sign(tail.vy) == sign(now.vy);
			}

			// An empty walk at the end is taken over rather than kept.
			void append(const Movement &element)
			{
				if (!elements.empty() && elements.back().command == 0 && elements.back().duration == 0)
					elements.back() = element;
				else
					elements.push_back(element);
			}

			std::vector<Movement> elements;
			int32_t recorded = 0;
			Point<int16_t> start;
			Movement last;
		};

		// Plays another character's paths back in order, as received.
		class Player
		{
		public:
			struct State
			{
				double x;
				double y;
				// Pixels per second.
				double vx;
				double vy;
				int16_t fh;
				uint8_t stance;
			};

			explicit Player(State state) : state(state), from(state)
			{}

			// Queue a path. One that falls more than five seconds behind is
			// dropped for a jump to where it ends.
			void receive(const std::vector<Movement> &elements)
			{
				if (elements.empty())
					return;

				for (const Movement &element : elements)
				{
					queue.push_back(element);
					queued += element.duration * TICK;
				}

				rate = queued < BEHIND * TICK ? TICK : FAST;

				if (queued > (BEHIND + 5000) * TICK)
				{
					Movement snap = elements.back();
					snap.command = 3;
					snap.duration = 0;

					queue.assign(1, snap);
					queued = 0;
				}
			}

			// Advance `ms` milliseconds, a tenth faster while a second or
			// more is queued.
			void step(int32_t ms)
			{
				if (queue.empty())
					return;

				int32_t advance = ms * rate;
				elapsed += advance;

				while (!queue.empty())
				{
					const Movement &head = queue.front();

					if (head.command != 10)
					{
						if (elapsed < head.duration * TICK)
							break;

						elapsed -= head.duration * TICK;
						arrive(head);
					}

					queue.pop_front();
				}

				queued -= advance;

				if (queue.empty() || queued <= 0)
				{
					queue.clear();
					elapsed = 0;
					queued = 0;

					return;
				}

				const Movement &head = queue.front();
				double x = state.x;
				double y = state.y;

				if (head.command == 3)
				{
					state.x = head.xpos;
					state.y = head.ypos;
					state.vx = head.vx;
					state.vy = head.vy;
				} else
				{
					state.x = curve(from.x, from.vx, head.xpos, head.vx, head.duration);
					state.y = curve(from.y, from.vy, head.ypos, head.vy, head.duration);
					state.vx = (state.x - x) * 1000 / ms;
					state.vy = (state.y - y) * 1000 / ms;
				}

				state.fh = head.fh;
				state.stance = head.newstate;
			}

			// Stop at the first element on a foothold from the one playing
			// now, dropping the rest. A flier, or one with none ahead,
			// stops where it is (FUN_0068b16f).
			void land(bool flying)
			{
				auto stop = flying ? queue.end() : std::find_if(queue.begin(), queue.end(), grounded);

				if (stop == queue.end())
				{
					queue.clear();
					from = state;
					elapsed = 0;
					queued = 0;

					return;
				}

				for (auto element = stop + 1; element != queue.end(); ++element)
					queued -= element->duration * TICK;

				queue.erase(stop + 1, queue.end());
			}

			// Where the queued paths end, or the state with none.
			State end() const
			{
				if (queue.empty())
					return state;

				const Movement &tail = queue.back();

				return { static_cast<double>(tail.xpos), static_cast<double>(tail.ypos), static_cast<double>(tail.vx),
						 static_cast<double>(tail.vy), tail.fh, tail.newstate };
			}

			const State &get_state() const
			{
				return state;
			}

			bool playing() const
			{
				return !queue.empty();
			}

		private:
			// Times are kept in thirtieths of a millisecond, so the faster
			// rate adds up exactly.
			static constexpr int32_t TICK = 30;
			static constexpr int32_t FAST = 33;
			static constexpr int32_t BEHIND = 1100;

			void arrive(const Movement &element)
			{
				state.x = element.xpos;
				state.y = element.ypos;
				state.vx = element.vx;
				state.vy = element.vy;
				state.fh = element.fh;
				state.stance = element.newstate;
				from = state;
			}

			// The cubic Hermite curve from where the last element ended to
			// where this one ends, with both speeds (FUN_0068b108).
			double curve(double p0, double v0, double p1, double v1, int16_t duration) const
			{
				double t = static_cast<double>(elapsed) / (duration * TICK);
				double seconds = elapsed / (1000.0 * TICK);
				double t2 = t * t;
				double t3 = t2 * t;

				return (2 * t3 - 3 * t2 + 1) * p0 + (3 * t2 - 2 * t3) * p1 + (t2 - 2 * t + 1) * seconds * v0 +
					   (t2 - t) * seconds * v1;
			}

			State state;
			State from;
			std::deque<Movement> queue;
			int32_t elapsed = 0;
			int32_t queued = 0;
			int32_t rate = TICK;
		};
	}
}
