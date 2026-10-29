#include "MiniRoomBalloon.h"

#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <cstdio>

namespace ms
{
	namespace
	{
		constexpr int8_t OMOK = 1;
		constexpr int8_t MATCH_CARDS = 2;
		constexpr int8_t PERSONAL_SHOP = 4;
		constexpr int8_t HIRED_MERCHANT = 5;

		// The title wraps once at 100 and the rest is cut short
		// (FUN_0048dcbd); the seat sign sits in the white box.
		constexpr uint16_t TITLE_WIDTH = 100;
		constexpr Point<int16_t> FIRST_LINE = Point<int16_t>(8, 8);
		constexpr Point<int16_t> SECOND_LINE = Point<int16_t>(8, 22);
		constexpr Point<int16_t> SEATS = Point<int16_t>(60, 39);
		// A game's icon, lock and seats, which the Progress sign replaces
		// while it is played (report b164-game-balloon).
		constexpr Point<int16_t> ICON = Point<int16_t>(7, 40);
		constexpr Point<int16_t> LOCK = Point<int16_t>(30, 39);
		constexpr Point<int16_t> USERS = Point<int16_t>(70, 42);
		constexpr Point<int16_t> MAX_USERS = Point<int16_t>(90, 42);
		// v83 draws a line's cap top about 3 below where it is drawn, we 9.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		// A bubble floats this far over the head; a signboard stands 2 below
		// the feet (FUN_0048e85a).
		constexpr int16_t BUBBLE_GAP = 5;
		constexpr int16_t SIGN_DROP = 2;
	}

	MiniRoomBalloon::MiniRoomBalloon() : shown(false), serial(0), signboard(false), game(false), locked(false)
	{}

	void MiniRoomBalloon::change(const std::optional<MiniRoomBox> &box)
	{
		shown = box && (box->type == OMOK || box->type == MATCH_CARDS || box->type == PERSONAL_SHOP
				|| box->type == HIRED_MERCHANT);

		if (!shown)
			return;

		nl::node miniroom = NxFiles::ui_classic["ChatBalloon.img"]["miniroom"];

		serial = box->serial;
		game = box->type == OMOK || box->type == MATCH_CARDS;
		locked = game && box->locked;
		icon = Texture();
		lock = Texture();
		users = Texture();
		max_users = Texture();

		if (game)
		{
			// Match cards show their deck's back; neither game has
			// Able/Disable (0x48c86d).
			signboard = false;
			skin = miniroom["backgrnd"];
			icon = box->type == OMOK ? miniroom["Omok"] : miniroom["MemoryGame"][std::to_string(box->skin)];
			lock = miniroom[box->locked ? "Lock" : "Unlock"];
			seats = Texture();

			if (box->in_progress)
			{
				seats = miniroom["Progress"];
			}
			else
			{
				users = miniroom["cNum"][std::to_string(box->users)];
				max_users = miniroom["mNum"][std::to_string(box->max_users)];
			}
		}
		else if (box->type == HIRED_MERCHANT)
		{
			// The permit's signboard, `050300%02d/skin` (SP 0xd80, 0xd82).
			// The merchant's own seat counts, so one seat less is free
			// (0x48c788).
			char id[16];
			std::snprintf(id, sizeof(id), "050300%02d", box->skin);
			signboard = true;
			skin = nl::nx::item["Cash"]["0503.img"][id]["skin"]["signboard"];

			if (!skin.is_valid())
				skin = nl::nx::item["Cash"]["0503.img"]["05030000"]["skin"]["signboard"];

			seats = miniroom[box->users + 1 < box->max_users ? "Able" : "Disable"];
		}
		else
		{
			signboard = box->skin != 0;
			skin = miniroom["PSSkin"][std::to_string(box->skin)];

			if (!skin.is_valid())
			{
				signboard = false;
				skin = miniroom["PSSkin"]["0"];
			}

			seats = miniroom[box->users < box->max_users ? "Able" : "Disable"];
		}

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
		cut_title = second.get_text() == box->title.substr(fits) ? "" : box->title;
	}

	void MiniRoomBalloon::draw(Point<int16_t> feet, int16_t avatar_height) const
	{
		if (!shown)
			return;

		Point<int16_t> top_left = bounds(feet, avatar_height).get_left_top();
		// Text and the seat sign are placed from the skin's origin.
		Point<int16_t> origin = top_left + skin.get_origin();

		skin.draw(origin);
		icon.draw(origin + ICON);
		lock.draw(origin + LOCK);
		seats.draw(origin + SEATS);
		users.draw(origin + USERS);
		max_users.draw(origin + MAX_USERS);
		first.draw(origin + FIRST_LINE + TEXT_Y);
		second.draw(origin + SECOND_LINE + TEXT_Y);
	}

	int32_t MiniRoomBalloon::serial_at(Point<int16_t> feet, int16_t avatar_height, Point<int16_t> cursor) const
	{
		std::optional<MiniRoomHit> hit = hit_at(feet, avatar_height, cursor);

		return hit ? hit->serial : 0;
	}

	std::optional<MiniRoomHit> MiniRoomBalloon::hit_at(Point<int16_t> feet, int16_t avatar_height,
			Point<int16_t> cursor) const
	{
		if (shown && bounds(feet, avatar_height).contains(cursor))
			return MiniRoomHit{ serial, game, locked, cut_title };

		return std::nullopt;
	}

	Rectangle<int16_t> MiniRoomBalloon::bounds(Point<int16_t> feet, int16_t avatar_height) const
	{
		Point<int16_t> size = skin.get_dimensions();
		int16_t top = signboard ? SIGN_DROP - size.y() : -size.y() - avatar_height - BUBBLE_GAP;
		Point<int16_t> top_left = feet + Point<int16_t>(-size.x() / 2, top);

		return Rectangle<int16_t>(top_left, top_left + size);
	}
}
