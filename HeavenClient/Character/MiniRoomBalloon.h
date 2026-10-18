#pragma once

#include "MiniRoomBox.h"

#include "../Graphics/Text.h"
#include "../Graphics/Texture.h"
#include "../Template/Rectangle.h"

#include <optional>

namespace ms
{
	// The board over someone keeping a personal shop open (v83
	// FUN_0048bbca): the permit's skin with the store's title and whether a
	// seat is free. Double-clicking it visits the shop.
	class MiniRoomBalloon
	{
	public:
		MiniRoomBalloon();

		// Shows `box`, or nothing.
		void change(const std::optional<MiniRoomBox> &box);

		void draw(Point<int16_t> feet, int16_t avatar_height) const;

		// The room to visit when the balloon is clicked at `cursor`, or 0.
		int32_t serial_at(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const;

	private:
		Rectangle<int16_t> bounds(Point<int16_t> feet, int16_t avatar_height) const;

		bool shown;
		int32_t serial;
		// Skin 0 is a speech bubble over the head; the others are signboards
		// standing on the ground.
		bool signboard;
		Texture skin;
		Texture seats;
		Text first;
		Text second;
	};
}
