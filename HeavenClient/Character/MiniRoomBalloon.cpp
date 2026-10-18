#include "MiniRoomBalloon.h"

#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		constexpr int8_t PERSONAL_SHOP = 4;

		// The title wraps once at 100 and the rest is cut short
		// (FUN_0048dcbd); the seat sign sits in the white box.
		constexpr uint16_t TITLE_WIDTH = 100;
		constexpr Point<int16_t> FIRST_LINE = Point<int16_t>(8, 8);
		constexpr Point<int16_t> SECOND_LINE = Point<int16_t>(8, 22);
		constexpr Point<int16_t> SEATS = Point<int16_t>(60, 39);
		// v83 draws a line's cap top about 3 below where it is drawn, we 9.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		// A bubble floats this far over the head; a signboard stands 2 below
		// the feet (FUN_0048e85a).
		constexpr int16_t BUBBLE_GAP = 5;
		constexpr int16_t SIGN_DROP = 2;
	}

	MiniRoomBalloon::MiniRoomBalloon() : shown(false), serial(0), signboard(false)
	{}

	void MiniRoomBalloon::change(const std::optional<MiniRoomBox> &box)
	{
		// Only shops have balloons here; games have none yet.
		shown = box && box->type == PERSONAL_SHOP;

		if (!shown)
			return;

		nl::node miniroom = NxFiles::ui_classic["ChatBalloon.img"]["miniroom"];

		serial = box->serial;
		signboard = box->skin != 0;
		skin = miniroom["PSSkin"][std::to_string(box->skin)];

		if (!skin.is_valid())
		{
			signboard = false;
			skin = miniroom["PSSkin"]["0"];
		}

		seats = miniroom[box->users < box->max_users ? "Able" : "Disable"];

		// v83 breaks the title where the first line runs out, not at a word.
		Text measure = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		size_t fits = 0;

		while (fits < box->title.size())
		{
			measure.change_text(box->title.substr(0, fits + 1));

			if (measure.width() > TITLE_WIDTH)
				break;

			fits++;
		}

		first = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, box->title.substr(0, fits), 0, false);
		second = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, box->title.substr(fits), 0, false);
		string_format::format_with_ellipsis(second, TITLE_WIDTH);
	}

	void MiniRoomBalloon::draw(Point<int16_t> feet, int16_t avatar_height) const
	{
		if (!shown)
			return;

		Point<int16_t> top_left = bounds(feet, avatar_height).get_left_top();
		// Text and the seat sign are placed from the skin's origin.
		Point<int16_t> origin = top_left + skin.get_origin();

		skin.draw(origin);
		seats.draw(origin + SEATS);
		first.draw(origin + FIRST_LINE + TEXT_Y);
		second.draw(origin + SECOND_LINE + TEXT_Y);
	}

	int32_t MiniRoomBalloon::serial_at(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const
	{
		if (shown && bounds(feet, avatar_height).contains(cursor))
			return serial;

		return 0;
	}

	Rectangle<int16_t> MiniRoomBalloon::bounds(Point<int16_t> feet, int16_t avatar_height) const
	{
		Point<int16_t> size = skin.get_dimensions();
		int16_t top = signboard ? SIGN_DROP - size.y() : -size.y() - avatar_height - BUBBLE_GAP;
		Point<int16_t> top_left = feet + Point<int16_t>(-size.x() / 2, top);

		return Rectangle<int16_t>(top_left, top_left + size);
	}
}
