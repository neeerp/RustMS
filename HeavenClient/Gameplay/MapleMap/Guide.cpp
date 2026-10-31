#include "Guide.h"

#include "GuideRules.h"

#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <cmath>

namespace ms
{
	using namespace guide_rules;

	void Guide::spawn(int32_t id, Point<int16_t> position)
	{
		skill = id;

		nl::node summon = nl::nx::skill[string_format::extend_id(skill / 10000, 3) + ".img"]["skill"]
		[string_format::extend_id(skill, 7)]["summon"];
		nl::node tall = summon["height"];
		height = tall ? static_cast<int16_t>(tall.get_integer()) : DEFAULT_HEIGHT;

		nl::node frame = NxFiles::ui_classic["ChatBalloon.img"]["tutorial"];
		constexpr const char *names[NUM_PIECES] = { "nw", "n", "ne", "w", "c", "e", "sw", "s", "se", "arrow" };

		for (size_t i = 0; i < NUM_PIECES; i++)
			pieces[i] = Texture(frame[names[i]]);

		respawn(position);
	}

	void Guide::remove()
	{
		skill = 0;
		body.reset();
		speech = Speech::NONE;
	}

	bool Guide::is_present() const
	{
		return body.has_value();
	}

	bool Guide::contains(Point<int16_t> position) const
	{
		return body && body->get_body().contains(position);
	}

	void Guide::respawn(Point<int16_t> position)
	{
		if (skill == 0)
			return;

		// Action 2, following, no assist, appearing (0x7a3366).
		body.emplace(0, skill, 1, position, 2, summon_rules::FOLLOW, summon_rules::NOTHING, true, true);
	}

	void Guide::hush()
	{
		speech = Speech::NONE;
	}

	void Guide::say(const std::string &line, int32_t wrap, int32_t time)
	{
		// The last balloon or hint goes first (0x7a7b6a).
		speech = Speech::NONE;

		if (line.empty())
			return;

		speech = Speech::BALLOON;
		elapsed = 0;
		duration = time;
		width = static_cast<int16_t>(wrap);
		// The text renderer breaks lines at escaped breaks, not CR or LF.
		std::string lines;

		for (size_t i = 0; i < line.size(); i++)
		{
			if (line[i] == '\r' || line[i] == '\n')
			{
				if (line[i] == '\r' && i + 1 < line.size() && line[i + 1] == '\n')
					i++;

				lines += "\\n";
			} else
			{
				lines += line[i];
			}
		}

		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, lines, static_cast<uint16_t>(wrap));

		body->play(summon_rules::SAY);
	}

	void Guide::hint(int32_t id, int32_t time, bool talks)
	{
		speech = Speech::HINT;
		elapsed = 0;
		duration = time;
		picture = Animation(NxFiles::ui_classic["tutorial.img"][std::to_string(id)]);

		if (talks)
			body->play(summon_rules::SAY);
	}

	void Guide::update(const Physics &physics, Point<int16_t> owner, bool held)
	{
		if (!body)
			return;

		// Its path is its own; nobody else sees it.
		body->update(physics, owner, held, random);

		if (speech == Speech::NONE)
			return;

		elapsed += Constants::TIMESTEP;

		if (expired(elapsed, duration))
			speech = Speech::NONE;
		else if (speech == Speech::HINT)
			picture.update();
	}

	void Guide::draw(double viewx, double viewy, float alpha) const
	{
		if (!body)
			return;

		body->draw(viewx, viewy, alpha);

		// Rounded as the body is, so the balloon keeps to it while the view
		// scrolls.
		Point<int16_t> position = body->get_position();
		Point<int16_t> foot(static_cast<int16_t>(std::lround(position.x() + viewx)), static_cast<int16_t>(std::lround(position.y() + viewy)));

		if (speech == Speech::BALLOON)
		{
			draw_balloon(foot);
		} else if (speech == Speech::HINT)
		{
			// The picture's top left is placed; its origin plays no part.
			Point<int16_t> corner = foot + hint_corner(picture.get_dimensions(), height);
			picture.draw(DrawArgument(corner + picture.get_origin()), alpha);
		}
	}

	void Guide::draw_balloon(Point<int16_t> foot) const
	{
		int16_t lines = text.height();
		Point<int16_t> corner = foot + balloon_corner(width, lines, height);

		// Each piece by its top left.
		auto put = [&](Piece piece, int16_t x, int16_t y, int16_t w, int16_t h)
		{
			const Texture &texture = pieces[piece];
			pieces[piece].draw(DrawArgument(corner + Point<int16_t>(x, y) + texture.get_origin(), Point<int16_t>(w, h)));
		};

		int16_t right = SIDE + width;
		int16_t bottom = TOP + lines;

		put(NW, 0, 0, 0, 0);
		put(N, SIDE, 0, width, 0);
		put(NE, right, 0, 0, 0);
		put(W, 0, TOP, 0, lines);
		put(C, SIDE, TOP, width, lines);
		put(E, right, TOP, 0, lines);
		put(SW, 0, bottom, 0, 0);
		put(S, SIDE, bottom, width, 0);
		put(SE, right, bottom, 0, 0);

		Point<int16_t> arrow = balloon_arrow(width, lines);
		put(ARROW, arrow.x(), arrow.y(), 0, 0);

		text.draw(corner + Point<int16_t>(SIDE, TOP));
	}
}
