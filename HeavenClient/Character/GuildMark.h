#pragma once

#include "Guild.h"

#include "../Graphics/Texture.h"

namespace ms
{
	// A guild emblem from GuildMark.img: the mark over its background,
	// 17 pixels square. Draws nothing for an emblem the client hides.
	class GuildMark
	{
	public:
		static constexpr int16_t SIZE = 17;

		GuildMark(const GuildEmblem& emblem);
		GuildMark();

		// Draws with the top-left corner at pos.
		void draw(Point<int16_t> pos) const;

	private:
		Texture background, mark;
	};
}
