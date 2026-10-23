#pragma once

#include "MiniRoomBox.h"

#include "../Graphics/Text.h"
#include "../Graphics/Texture.h"
#include "../Template/Rectangle.h"

#include <optional>

namespace ms
{
	// The board over someone keeping a personal shop or a game open (v83
	// FUN_0048bbca): the permit's skin or the game's, with the room's title
	// and whether a seat is free. Double-clicking it visits the room.
	class MiniRoomBalloon
	{
	public:
		MiniRoomBalloon();

		// Shows `box`, or nothing.
		void change(const std::optional<MiniRoomBox> &box);

		void draw(Point<int16_t> feet, int16_t avatar_height) const;

		// The room to visit when the balloon is clicked at `cursor`, or 0.
		int32_t serial_at(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const;

		// The room under `cursor`, if the balloon is.
		std::optional<MiniRoomHit> hit_at(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const;

	private:
		Rectangle<int16_t> bounds(Point<int16_t> feet, int16_t avatar_height) const;

		bool shown;
		int32_t serial;
		// Skin 0 is a speech bubble over the head; the others are signboards
		// standing on the ground.
		bool signboard;
		bool game;
		bool locked;
		Texture skin;
		// A shop's seat sign, or a game's sign that it is being played.
		Texture seats;
		// A game's icon, lock and seat counts.
		Texture icon;
		Texture lock;
		Texture users;
		Texture max_users;
		Text first;
		Text second;
	};
}
