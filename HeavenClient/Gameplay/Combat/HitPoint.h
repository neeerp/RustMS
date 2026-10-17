#pragma once

#include "../../Template/Rectangle.h"

#include <algorithm>
#include <cstdint>
#include <optional>

namespace ms
{
	// Where a hit lands on a mob, and where a skill's hit effect is drawn,
	// as in the MapleRoyals client. Rectangles and points are in map
	// coordinates.
	namespace hit_point
	{
		// The middle of the overlap of the attack and the body
		// (FUN_00664603), first stretching an attack that falls short to
		// touch the body by a pixel.
		inline Point<int16_t> centre(const Rectangle<int16_t> &range, const Rectangle<int16_t> &body)
		{
			int16_t left = range.left();
			int16_t right = range.right();
			int16_t top = range.top();
			int16_t bottom = range.bottom();

			if (left >= body.right())
				left = body.right() - 1;
			else if (right <= body.left())
				right = body.left() + 1;

			if (top >= body.bottom())
				top = body.bottom() - 1;
			else if (bottom <= body.top())
				bottom = body.top() + 1;

			return {
					static_cast<int16_t>((std::max(left, body.left()) + std::min(right, body.right())) / 2),
					static_cast<int16_t>((std::max(top, body.top()) + std::min(bottom, body.bottom())) / 2)
			};
		}

		// What a hit effect's `pos` attaches it to (FUN_0066936e).
		enum class Anchor
		{
			// The mob's feet, following it.
			FEET,
			// The current frame's head, following the mob and its frames.
			HEAD,
			// A point in the field that stays put if the mob moves.
			FIELD
		};

		struct Placement
		{
			Anchor anchor;
			// From the feet for FEET, from the head for HEAD, and the point
			// itself for FIELD.
			Point<int16_t> offset;
		};

		// An effect needs a body to land on, except at the feet or head.
		inline bool hollow(const Rectangle<int16_t> &body)
		{
			return body.right() <= body.left() || body.bottom() <= body.top();
		}

		inline std::optional<Placement> place(int8_t pos, const Rectangle<int16_t> &body, Point<int16_t> mob,
											   Point<int16_t> hit)
		{
			switch (pos)
			{
				case 1:
					return Placement{ Anchor::FEET, {}};
				case 2:
					return Placement{ Anchor::HEAD, {}};
				case 3:
					if (hollow(body))
						return std::nullopt;

					return Placement{
							Anchor::FEET,
							Point<int16_t>(
									static_cast<int16_t>((body.left() + body.right()) / 2 - mob.x()),
									static_cast<int16_t>((body.top() + body.bottom()) / 2 - mob.y())
							)
					};
				case 4:
					return Placement{ Anchor::FEET, hit - mob };
				default:
					if (hollow(body))
						return std::nullopt;

					return Placement{
							Anchor::FIELD,
							Point<int16_t>(
									std::clamp(hit.x(), body.left(), body.right()),
									std::clamp(hit.y(), body.top(), body.bottom())
							)
					};
			}
		}
	}
}
