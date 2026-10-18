#pragma once

#include "../Graphics/Text.h"
#include "../Graphics/Texture.h"
#include "../Template/Rectangle.h"

#include <array>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The board over someone using a chalkboard (v83 FUN_004890c4 with the
	// ChatBalloon.img "adboard" style): white text on a tiled board that stays
	// up until the server takes it down. The player's own board has a close
	// button.
	class ChalkboardBalloon
	{
	public:
		ChalkboardBalloon();

		// Shows `text`, or nothing.
		void change(const std::optional<std::string> &text, bool closable);

		bool is_shown() const;

		const std::string &get_text() const;

		void draw(Point<int16_t> feet, int16_t avatar_height) const;

		// Tracks the close button under the cursor. True when a left click on
		// it is released over it.
		bool send_cursor(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor, bool pressed, bool held);

		// Forgets a press on the close button whose release went elsewhere.
		void release_cursor();

		// Whether the cursor is over the close button.
		bool on_close(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const;

	private:
		Point<int16_t> top_left(Point<int16_t> feet, int16_t avatar_height) const;

		Rectangle<int16_t> close_bounds(Point<int16_t> feet, int16_t avatar_height) const;

		// The board pieces, named in ChatBalloon.img as c, n, s, w, e, nw, ne, sw, se, and arrow.
		enum Piece : uint8_t
		{
			CENTER, NORTH, SOUTH, WEST, EAST, NORTHWEST, NORTHEAST, SOUTHWEST, SOUTHEAST, ARROW, NUM_PIECES
		};

		enum CloseState : uint8_t
		{
			NORMAL, MOUSEOVER, PRESSED, NUM_STATES
		};

		bool shown;
		bool closable;
		bool armed;
		CloseState close_state;
		std::string text;
		std::vector<Text> lines;
		std::array<Texture, NUM_PIECES> pieces;
		std::array<Texture, NUM_STATES> close;
		// The frame's margins around the inner area.
		int16_t left;
		int16_t right;
		int16_t top;
		int16_t bottom;
		int16_t tiles;
		Point<int16_t> size;
	};
}
