#include "ChalkboardBalloon.h"

#include "../Util/Chalkboard.h"
#include "../Util/NxFiles.h"

#include <algorithm>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The board floats this far over the head, its arrow 3 right of the
		// feet (FUN_0048e298).
		constexpr int16_t GAP = 5;
		constexpr int16_t ARROW_SHIFT = 3;
		// The close button sits 16 from the board's right and 4 from its top.
		constexpr Point<int16_t> CLOSE_OFFSET = Point<int16_t>(-16, 4);
		// The balloon font's descent; see ChatBalloon.
		constexpr int16_t TEXT_DESCENT = 3;
	}

	ChalkboardBalloon::ChalkboardBalloon() : shown(false), closable(false), armed(false), close_state(NORMAL), left(0),
		right(0), top(0), bottom(0), tiles(0)
	{}

	void ChalkboardBalloon::change(const std::optional<std::string> &note, bool can_close)
	{
		shown = note.has_value();
		closable = can_close;
		armed = false;
		close_state = NORMAL;
		text = note.value_or("");
		lines.clear();

		if (!shown)
			return;

		nl::node adboard = NxFiles::ui_classic["ChatBalloon.img"]["adboard"]["0"];

		constexpr const char *names[NUM_PIECES] = { "c", "n", "s", "w", "e", "nw", "ne", "sw", "se", "arrow" };

		for (size_t i = 0; i < NUM_PIECES; i++)
			pieces[i] = adboard[names[i]];

		nl::node button = NxFiles::ui_classic["Basic.img"]["BtClose"];
		close[NORMAL] = button["normal"]["0"];
		close[MOUSEOVER] = button["mouseOver"]["0"];
		close[PRESSED] = button["pressed"]["0"];

		// The margins reach as far as the pieces stick out of the inner area.
		auto ox = [this](Piece p) { return pieces[p].get_origin().x(); };
		auto oy = [this](Piece p) { return pieces[p].get_origin().y(); };
		auto w = [this](Piece p) { return pieces[p].get_dimensions().x(); };
		auto h = [this](Piece p) { return pieces[p].get_dimensions().y(); };

		left = std::max({ ox(SOUTHWEST), ox(NORTHWEST), ox(WEST) });
		right = std::max({ w(SOUTHEAST) - ox(SOUTHEAST), w(NORTHEAST) - ox(NORTHEAST), w(EAST) - ox(EAST) });
		top = std::max({ oy(NORTHEAST), oy(NORTHWEST), oy(NORTH) });
		bottom = std::max(h(ARROW) - oy(ARROW), h(SOUTH) - oy(SOUTH));

		Text measure(Text::Font::BALLOON, Text::Alignment::LEFT, Color::Name::WHITE, "", 0, false);
		auto width = [&measure](const std::string &part)
		{
			measure.change_text(part);

			return measure.width();
		};

		for (const std::string &line : Chalkboard::wrap(text, width))
			lines.emplace_back(Text::Font::BALLOON, Text::Alignment::LEFT, Color::Name::WHITE, line, 0, false);

		int16_t first_width = lines.empty() ? 0 : lines.front().width();
		tiles = Chalkboard::tiles(lines.size(), first_width);

		int16_t rows = static_cast<int16_t>(lines.size());
		size = Point<int16_t>(Chalkboard::TILE * tiles + left + right, Chalkboard::TILE * rows + top + bottom);
	}

	bool ChalkboardBalloon::is_shown() const
	{
		return shown;
	}

	const std::string &ChalkboardBalloon::get_text() const
	{
		return text;
	}

	Point<int16_t> ChalkboardBalloon::top_left(Point<int16_t> feet, int16_t avatar_height) const
	{
		int16_t inner = Chalkboard::TILE * tiles;

		return feet + Point<int16_t>(ARROW_SHIFT - inner / 2 - right, -size.y() - avatar_height - GAP);
	}

	Rectangle<int16_t> ChalkboardBalloon::close_bounds(Point<int16_t> feet, int16_t avatar_height) const
	{
		Point<int16_t> lt = top_left(feet, avatar_height) + Point<int16_t>(size.x(), 0) + CLOSE_OFFSET;

		return Rectangle<int16_t>(lt, lt + close[NORMAL].get_dimensions());
	}

	void ChalkboardBalloon::draw(Point<int16_t> feet, int16_t avatar_height) const
	{
		if (!shown)
			return;

		// Pieces are placed through their origins, as in the original.
		Point<int16_t> origin = top_left(feet, avatar_height);
		int16_t inner_right = size.x() - right;
		int16_t inner_bottom = size.y() - bottom;
		auto at = [origin](int16_t x, int16_t y) { return DrawArgument(origin + Point<int16_t>(x, y)); };

		for (int16_t i = 0; i < tiles; i++)
		{
			int16_t x = left + Chalkboard::TILE * i;

			pieces[NORTH].draw(at(x, top));
			pieces[i == tiles / 2 ? ARROW : SOUTH].draw(at(x, inner_bottom));
		}

		for (size_t j = 0; j < lines.size(); j++)
		{
			int16_t y = top + Chalkboard::TILE * static_cast<int16_t>(j);

			pieces[WEST].draw(at(left, y));

			for (int16_t k = 0; k < tiles; k++)
				pieces[CENTER].draw(at(left + Chalkboard::TILE * k, y));

			pieces[EAST].draw(at(inner_right, y));
		}

		pieces[NORTHWEST].draw(at(left, top));
		pieces[NORTHEAST].draw(at(inner_right, top));
		pieces[SOUTHWEST].draw(at(left, inner_bottom));
		pieces[SOUTHEAST].draw(at(inner_right, inner_bottom));

		int16_t inner = Chalkboard::TILE * tiles;

		for (size_t j = 0; j < lines.size(); j++)
		{
			int16_t x = left + (inner - lines[j].width()) / 2;
			int16_t y = top + Chalkboard::TILE * static_cast<int16_t>(j) - TEXT_DESCENT;

			lines[j].draw(at(x, y));
		}

		if (closable)
			close[close_state].draw(DrawArgument(close_bounds(feet, avatar_height).get_left_top()));
	}

	bool ChalkboardBalloon::on_close(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const
	{
		return shown && closable && close_bounds(feet, avatar_height).contains(cursor);
	}

	bool ChalkboardBalloon::send_cursor(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor, bool pressed,
										bool held)
	{
		bool inside = on_close(feet, avatar_height, cursor);
		bool closed = false;

		if (pressed)
		{
			armed = inside;
		} else if (!held && armed)
		{
			armed = false;
			closed = inside;
		}

		close_state = !inside ? NORMAL : armed && held ? PRESSED : MOUSEOVER;

		return closed;
	}

	void ChalkboardBalloon::release_cursor()
	{
		armed = false;
	}
}
