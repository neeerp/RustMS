#include "UIAranCombo.h"

#include "../UI.h"

#include "../../Constants.h"
#include "../../Gameplay/Combat/AranCombo.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where FUN_00960342 puts the layers, on the screen: the ones digit, each digit before it 23 to the
		// left, the word, and the ComboCommand's lines, the second under the first.
		const int16_t DIGIT_X = 750;
		const int16_t DIGIT_ADVANCE = 23;
		const Point<int16_t> WORD(750, 180);
		const Point<int16_t> FIRST_LINE(800, 215);
		const Point<int16_t> SECOND_LINE(800, 250);
	}

	UIAranCombo::UIAranCombo()
			: UIElement(Point<int16_t>(), Point<int16_t>()), count(0), first_shown(false), second_shown(false),
			  elapsed(0), held(false)
	{
	}

	void UIAranCombo::show(int32_t count, bool first, bool second)
	{
		if (count <= 0)
		{
			UI::get().remove(TYPE);
			return;
		}

		if (!UI::get().get_element<UIAranCombo>())
			UI::get().emplace<UIAranCombo>();

		if (auto hud = UI::get().get_element<UIAranCombo>())
			hud->set(count, first, second);
	}

	void UIAranCombo::hold()
	{
		if (auto hud = UI::get().get_element<UIAranCombo>())
		{
			hud->set(hud->count, false, false);
			hud->held = true;
		}
	}

	void UIAranCombo::set(int32_t c, bool first, bool second)
	{
		count = c;
		elapsed = 0;
		held = false;

		nl::node src = nl::nx::effect["BasicEff.img"];
		nl::node tier = src["Combo"][std::to_string(aran_combo::tier(count))];

		digits.clear();

		for (int32_t digit : aran_combo::digits(count))
			digits.emplace_back(tier[std::to_string(digit)]["0"]);

		word = tier["Combo"];
		word.play(Animation::End::HOLD);

		// A count that announces nothing leaves the lines as they were.
		aran_combo::Command command = aran_combo::command(count);
		nl::node lines = src["ComboCommand"][std::to_string(command.index)];

		if (first)
		{
			first_line = lines["0"];
			first_line.play(Animation::End::HOLD);
			first_shown = true;
		}

		if (second)
		{
			second_line = lines["1"];
			second_line.play(Animation::End::HOLD);
			second_shown = true;
		}
	}

	void UIAranCombo::draw(float alpha) const
	{
		// The digits and the word fade from the moment the count comes until it ends.
		float opacity = held ? 1.0f : 1.0f - static_cast<float>(elapsed) / aran_combo::IDLE;

		if (opacity < 0.0f)
			opacity = 0.0f;

		for (size_t i = 0; i < digits.size(); i++)
		{
			int16_t x = DIGIT_X - static_cast<int16_t>(DIGIT_ADVANCE * i);
			int32_t down = held ? 0 : aran_combo::bounce(aran_combo::bounce_height(count, i), elapsed);
			int16_t y = aran_combo::digit_y(i, digits.size()) + static_cast<int16_t>(down);
			digits[i].draw(DrawArgument(Point<int16_t>(x, y), opacity));
		}

		word.draw(DrawArgument(WORD, opacity), alpha);

		// The second line takes the first's place without it.
		if (first_shown)
			first_line.draw(FIRST_LINE, alpha);

		if (second_shown)
			second_line.draw(first_shown ? SECOND_LINE : FIRST_LINE, alpha);
	}

	void UIAranCombo::update()
	{
		elapsed += Constants::TIMESTEP;
		word.update();

		if (first_shown)
			first_line.update();

		if (second_shown)
			second_line.update();
	}

	bool UIAranCombo::is_in_range(Point<int16_t>) const
	{
		return false;
	}

	UIElement::Type UIAranCombo::get_type() const
	{
		return TYPE;
	}
}
