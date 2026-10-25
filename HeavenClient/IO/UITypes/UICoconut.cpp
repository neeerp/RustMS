#include "UICoconut.h"

#include "../UI.h"

#include "../../Character/Coconut.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where the Init (FUN_00548b77) moves the 258x101 layer, on the screen.
		const Point<int16_t> BOARD(535, 8);
		// Where the redraw (FUN_00549a13) writes the scores and the time on it.
		const Point<int16_t> MAPLE(25, 37);
		const Point<int16_t> STORY(150, 37);
		const Point<int16_t> TIME(60, 83);
		// The space it leaves after each glyph of the scores and of the time.
		constexpr int16_t SCORE_GAP = 4;
		constexpr int16_t TIME_GAP = 1;

		std::map<char, Texture> font(nl::node src, const std::string& extra)
		{
			std::map<char, Texture> glyphs;

			for (char digit = '0'; digit <= '9'; digit++)
				glyphs[digit] = src[std::string(1, digit)];

			for (char glyph : extra)
				glyphs[glyph] = src[std::string(1, glyph)];

			return glyphs;
		}
	}

	UICoconut::UICoconut() : UIElement(Point<int16_t>(), Point<int16_t>())
	{
		nl::node src = nl::nx::map["Obj"]["etc.img"]["coconut"];

		backgrnd = src["backgrnd"];
		score_font = font(src["fontScore"], "");
		time_font = font(src["fontTime"], ":");
	}

	void UICoconut::show_on(int32_t fieldtype)
	{
		if (fieldtype == coconut::FIELD_TYPE)
		{
			if (!UI::get().get_element<UICoconut>())
				UI::get().emplace<UICoconut>();
		}
		else
		{
			UI::get().remove(TYPE);
		}
	}

	void UICoconut::draw(float) const
	{
		Point<int16_t> board = position + BOARD;
		backgrnd.draw(board + backgrnd.get_origin());

		const coconut::State& state = coconut::state();
		draw_text(score_font, coconut::score_text(state.score[0]), board + MAPLE, SCORE_GAP);
		draw_text(score_font, coconut::score_text(state.score[1]), board + STORY, SCORE_GAP);

		if (auto time = coconut::time_text(state.end, coconut::now()))
			draw_text(time_font, *time, board + TIME, TIME_GAP);
	}

	void UICoconut::draw_text(const std::map<char, Texture>& font, const std::string& text, Point<int16_t> pos,
							  int16_t gap) const
	{
		for (char glyph : text)
		{
			auto iter = font.find(glyph);

			if (iter == font.end())
				continue;

			const Texture& texture = iter->second;
			texture.draw(pos + texture.get_origin());
			pos.shift_x(texture.get_dimensions().x() + gap);
		}
	}

	bool UICoconut::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UICoconut::get_type() const
	{
		return TYPE;
	}
}
