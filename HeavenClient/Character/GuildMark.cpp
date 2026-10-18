#include "GuildMark.h"

#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	GuildMark::GuildMark(const GuildEmblem& emblem)
	{
		if (!emblem.shown())
			return;

		nl::node marks = NxFiles::ui_classic["GuildMark.img"];

		background = marks["BackGround"][string_format::extend_id(emblem.background, 8)]
			[std::to_string(emblem.background_color)];
		mark = marks["Mark"][guild_mark_category(emblem.mark)][string_format::extend_id(emblem.mark, 8)]
			[std::to_string(emblem.mark_color)];
	}

	GuildMark::GuildMark() {}

	void GuildMark::draw(Point<int16_t> pos) const
	{
		background.draw(pos);
		// The mark is centred on the background (FUN_00a3956e); letters are
		// narrower than the other marks.
		mark.draw(pos + (Point<int16_t>(SIZE, SIZE) - mark.get_dimensions()) / 2);
	}
}
