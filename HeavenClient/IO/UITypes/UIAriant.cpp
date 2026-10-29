#include "UIAriant.h"

#include "../UI.h"

#include "../../Audio/Audio.h"
#include "../../Character/Ariant.h"
#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Where the board's layer is, on the screen (0x53ea02).
		const Point<int16_t> BOARD(0, 30);
		// Its rows, 17 pixels apart: the icon, the name and "%4d" of the
		// score, the text 2 pixels below the icon's top.
		constexpr int16_t ROW_STEP = 17;
		constexpr int16_t ICON_X = 5;
		constexpr int16_t NAME_X = 21;
		constexpr int16_t SCORE_X = 106;
		constexpr int16_t TEXT_Y = 2;
		// v83 draws its text about 6 pixels higher than ours.
		constexpr int16_t TEXT_RAISE = 6;
		// Where the scroll's layer is, on the screen (0x53ef95).
		const Point<int16_t> RESULT(400, 100);
		// The text on it (0x53f14d), on a 320 pixel wide layer whose left
		// edge is the scroll's: the place from x 63, the name centred on 140
		// and the score on 235, the rows 26 pixels apart from y 104.
		const Point<int16_t> RESULT_TEXT(RESULT.x() - 160, RESULT.y());
		constexpr int16_t PLACE_X = 63;
		constexpr int16_t RESULT_NAME_X = 140;
		constexpr int16_t RESULT_SCORE_X = 235;
		constexpr int16_t RESULT_TOP = 104;
		constexpr int16_t RESULT_STEP = 26;
		// It fades in over a second.
		constexpr float RESULT_FADE = Constants::TIMESTEP / 1000.0f;

		bool same_rows(const std::vector<ariant::Row>& a, const std::vector<ariant::Row>& b)
		{
			return std::equal(a.begin(), a.end(), b.begin(), b.end(),
							  [](const ariant::Row& x, const ariant::Row& y)
							  { return x.name == y.name && x.score == y.score && x.icon == y.icon; });
		}
	}

	UIAriant::UIAriant() : UIElement(Point<int16_t>(), Point<int16_t>()), showing_result(false), result_written(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["AriantMatch"];

		for (size_t i = 0; i < icons.size(); i++)
			icons[i] = src["characterIcon"][std::to_string(i)];

		result = src["Result"];
		result_opacity.set(0.0f);
	}

	void UIAriant::show_on(int32_t fieldtype)
	{
		if (fieldtype == ariant::FIELD_TYPE)
		{
			if (!UI::get().get_element<UIAriant>())
				UI::get().emplace<UIAriant>();
		}
		else
		{
			UI::get().remove(TYPE);
		}
	}

	void UIAriant::show_result()
	{
		// Animate type 0: once, then the last frame stays.
		result.play(Animation::End::HOLD);
		showing_result = true;
		// Each 0x9B builds a new scroll, so the text is written again once it opens.
		result_written = false;
		result_places.clear();
		result_names.clear();
		result_scores.clear();
		Sound(nl::nx::sound["UI.img"]["ScrollOpen"]).play();
	}

	void UIAriant::draw(float alpha) const
	{
		// The results take the board away.
		for (size_t i = 0; i < shown.size() && !showing_result; i++)
		{
			const ariant::Row& row = shown[i];
			Point<int16_t> top = position + BOARD + Point<int16_t>(0, static_cast<int16_t>(ROW_STEP * i));

			// A seventh player's icon would be characterIcon/6, which isn't there.
			if (row.icon < icons.size())
				icons[row.icon].draw(top + Point<int16_t>(ICON_X, 0) + icons[row.icon].get_origin());

			Point<int16_t> text = top + Point<int16_t>(0, TEXT_Y - TEXT_RAISE);
			names[i].draw(text + Point<int16_t>(NAME_X, 0));
			scores[i].draw(text + Point<int16_t>(SCORE_X, 0));
		}

		if (showing_result)
			result.draw(position + RESULT, alpha);

		float opacity = result_opacity.get(alpha);

		for (size_t i = 0; i < result_names.size(); i++)
		{
			Point<int16_t> row = position + RESULT_TEXT +
								 Point<int16_t>(0, static_cast<int16_t>(RESULT_TOP + RESULT_STEP * i - TEXT_RAISE));

			result_places[i].draw(DrawArgument(row + Point<int16_t>(PLACE_X, 0), opacity));
			result_names[i].draw(DrawArgument(row + Point<int16_t>(RESULT_NAME_X, 0), opacity));
			result_scores[i].draw(DrawArgument(row + Point<int16_t>(RESULT_SCORE_X, 0), opacity));
		}
	}

	void UIAriant::update()
	{
		UIElement::update();

		const std::vector<ariant::Row>& rows = ariant::board().rows();

		if (!same_rows(rows, shown))
		{
			shown = rows;
			names.clear();
			scores.clear();

			// Font 0x26: bold 12 point, blue (#2000FF).
			for (const ariant::Row& row : shown)
			{
				names.emplace_back(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::GIFTBLUE, row.name);
				scores.emplace_back(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::GIFTBLUE,
									ariant::score_text(row.score));
			}
		}

		if (!showing_result)
			return;

		result.update();

		// Once the scroll is open its text is written, from the board as it
		// stands, and stays; a second ARIANT_RESULT doesn't write it again.
		if (!result_written && !result.is_playing())
		{
			std::vector<int32_t> places = ariant::places(rows);

			// Font 1: Arial 12, black.
			for (size_t i = 0; i < rows.size(); i++)
			{
				result_places.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
										   ariant::place_text(places[i]));
				result_names.emplace_back(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK,
										  rows[i].name);
				result_scores.emplace_back(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK,
										   std::to_string(rows[i].score));
			}

			result_written = true;
			result_opacity.set(0.0f);
		}
		else if (result_written)
		{
			result_opacity = std::min(1.0f, result_opacity.get() + RESULT_FADE);
		}
	}

	bool UIAriant::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIAriant::get_type() const
	{
		return TYPE;
	}
}
