#pragma once

#include "ShotPath.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <vector>

namespace ms
{
	// How Chain Lightning jumps from mob to mob and draws its bolt, as in the
	// MapleRoyals client (FUN_0095571f).
	namespace chain_lightning
	{
		// How far the bolt jumps from one mob to the next, across or up and
		// down, and how far across from the first mob it can go in all.
		constexpr int16_t HOP = 150;
		constexpr int16_t REACH = 400;

		// How many mobs near the last one it looks at.
		constexpr size_t LOOK = 10;

		// How long each part of the bolt shows, in ms.
		constexpr uint16_t BOLT_TIME = 270;

		// How far apart the bolt's balls are.
		constexpr int32_t BALL_SPACING = 48;

		// How many kinds of ball the bolt is made of.
		constexpr size_t BALL_KINDS = 3;

		// Where the bolt leaves the user from.
		constexpr int16_t HAND_REACH = 25;

		inline bool chains(int32_t skill)
		{
			return skill == 2221006;
		}

		// A mob the bolt might jump to.
		struct Mob
		{
			int32_t oid;
			Point<int16_t> position;
			Rectangle<int16_t> body;
			// When it entered the map: earlier ones are looked at first.
			uint32_t entry;
		};

		// The mobs the bolt hits, starting with `first` (FUN_0067886d). From
		// each, it looks at the first 10 living mobs to entry order whose
		// bodies are up to 150 px ahead and above or below it, but no more
		// than 400 px ahead of the first. It jumps to the nearest of those it
		// hasn't hit, the earliest on a tie, until it has hit `count` or finds
		// none.
		inline std::vector<int32_t> chain(const Mob &first, std::vector<Mob> mobs, uint8_t count, bool toleft)
		{
			std::stable_sort(mobs.begin(), mobs.end(), [](const Mob &a, const Mob &b) { return a.entry < b.entry; });

			std::vector<int32_t> targets = { first.oid };
			Mob last = first;

			while (targets.size() < count)
			{
				int32_t x = last.position.x();
				int32_t y = last.position.y();
				int32_t left = toleft ? std::max(x - HOP, first.position.x() - REACH) : x;
				int32_t right = toleft ? x : std::min(x + HOP, first.position.x() + REACH);
				Rectangle<int16_t> area(
					static_cast<int16_t>(left),
					static_cast<int16_t>(right),
					static_cast<int16_t>(y - HOP),
					static_cast<int16_t>(y + HOP)
				);

				const Mob *next = nullptr;
				int64_t nearest = INT64_MAX;
				size_t looked = 0;

				for (const Mob &mob : mobs)
				{
					if (looked == LOOK)
						break;

					if (mob.oid == last.oid || !shot_path::in_step(area, mob.body))
						continue;

					looked++;

					if (std::find(targets.begin(), targets.end(), mob.oid) != targets.end())
						continue;

					int64_t dx = mob.position.x() - x;
					int64_t dy = mob.position.y() - y;
					int64_t distance = dx * dx + dy * dy;

					if (distance < nearest)
					{
						nearest = distance;
						next = &mob;
					}
				}

				if (!next)
					break;

				targets.push_back(next->oid);
				last = *next;
			}

			return targets;
		}

		// Where the bolt leaves the user.
		inline Point<int16_t> start(Point<int16_t> user, bool toleft)
		{
			return {
				static_cast<int16_t>(user.x() + (toleft ? -HAND_REACH : HAND_REACH)),
				static_cast<int16_t>(user.y() - shot_path::HEIGHT)
			};
		}

		// The middle of a mob's body, where the bolt meets it.
		inline Point<int16_t> middle(Rectangle<int16_t> body)
		{
			return {
				static_cast<int16_t>((body.left() + body.right()) / 2),
				static_cast<int16_t>((body.top() + body.bottom()) / 2)
			};
		}

		// One ball of the bolt, turned to lie along it.
		struct Ball
		{
			Point<int16_t> position;
			int32_t degrees;
		};

		// The balls of one jump of the bolt (FUN_00443e60): one per 48 px of
		// its length and one more, 48 px apart and centred on its middle.
		inline std::vector<Ball> bolt(Point<int16_t> from, Point<int16_t> to)
		{
			int32_t x1 = from.x(), y1 = from.y(), x2 = to.x(), y2 = to.y();

			if (x1 > x2)
			{
				std::swap(x1, x2);
				std::swap(y1, y2);
			}

			int32_t degrees;

			if (x1 == x2)
				degrees = y1 >= y2 ? 270 : 90;
			else
				degrees = static_cast<int32_t>(std::atan(static_cast<double>(y2 - y1) / (x2 - x1)) * 57.29579143313326);

			int32_t length = static_cast<int32_t>(std::sqrt(static_cast<double>((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1))));
			int32_t count = length / BALL_SPACING + 1;
			int32_t half = count / 2;
			double across = std::cos(std::abs(degrees) * 2 * 0.008726644444444445);
			double down = std::sin(degrees * 2 * 0.008726644444444445);

			std::vector<Ball> balls;

			for (int32_t i = 0; i < count; i++)
			{
				double offset = (i - half) * static_cast<double>(BALL_SPACING);

				balls.push_back({
					{
						static_cast<int16_t>(across * offset + (x1 + x2) * 0.5),
						static_cast<int16_t>(down * offset + (y1 + y2) * 0.5)
					},
					degrees
				});
			}

			return balls;
		}
	}
}
