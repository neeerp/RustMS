#include "UIScriptMessage.h"

#include "../../Constants.h"

namespace ms
{
	namespace
	{
		// The first line's middle, from the top; later lines sit higher. v83's line is a 20 px layer around it.
		constexpr int16_t FIRST_Y = 150;
		constexpr int16_t LINE_HEIGHT = 20;
		constexpr int16_t HALF_LINE = 10;
		// Where the black copies go under the gold.
		const std::array<Point<int16_t>, 4> EDGE = {
			Point<int16_t>(-1, 0), Point<int16_t>(1, 0), Point<int16_t>(0, -1), Point<int16_t>(0, 1)
		};
	}

	UIScriptMessage::UIScriptMessage() : UIElement(Point<int16_t>(), Point<int16_t>())
	{
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIScriptMessage::draw(float) const
	{
		for (size_t i = 0; i < FloatingLines::SLOTS; i++)
		{
			float opacity = lines.opacity(i);

			if (opacity <= 0.0f)
				continue;

			Point<int16_t> at = position + Point<int16_t>(0, FIRST_Y - HALF_LINE - LINE_HEIGHT * static_cast<int16_t>(i));

			// v83 fades the outlined line as one layer. Drawn apart, edges at full fade would darken the gold
			// they lie under, so they fade faster.
			for (const Point<int16_t> &edge : EDGE)
				edges[i].draw(DrawArgument(at + edge, opacity * opacity));

			texts[i].draw(DrawArgument(at, opacity));
		}
	}

	void UIScriptMessage::update()
	{
		lines.update(Constants::TIMESTEP);
	}

	void UIScriptMessage::update_screen(int16_t new_width, int16_t)
	{
		position = Point<int16_t>(new_width / 2, 0);
	}

	bool UIScriptMessage::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIScriptMessage::get_type() const
	{
		return TYPE;
	}

	void UIScriptMessage::show(const std::string &text)
	{
		if (auto slot = lines.add(text))
		{
			// One plain line: no colour codes or breaks.
			texts[*slot] = Text(Text::Font::A15B, Text::Alignment::CENTER, Color::Name::SCRIPTGOLD, text, 0, false);
			edges[*slot] = Text(Text::Font::A15B, Text::Alignment::CENTER, Color::Name::BLACK, text, 0, false);
		}
	}
}
