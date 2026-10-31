#include "FocusRing.h"

#include "TabFocus.h"

#include "../UI.h"

namespace ms
{
	void FocusRing::add(Textfield &field)
	{
		stops.push_back({ &field, nullptr, 0 });
	}

	void FocusRing::add(uint16_t id, Button &button)
	{
		stops.push_back({ nullptr, &button, id });
	}

	void FocusRing::add_blank()
	{
		stops.push_back({ nullptr, nullptr, 0 });
	}

	void FocusRing::cycle(bool back)
	{
		auto to = tab_focus::next(current(), stops.size(), back, [this](size_t index)
		{
			return accepts(index);
		});

		if (to)
			focus(*to);
	}

	void FocusRing::focus_button(uint16_t id)
	{
		for (size_t i = 0; i < stops.size(); i++)
			if (stops[i].button && stops[i].id == id)
				focus(i);
	}

	void FocusRing::update()
	{
		for (const Stop &stop : stops)
			if (stop.field && stop.field->get_state() == Textfield::State::FOCUSED)
				let_go();
	}

	std::optional<uint16_t> FocusRing::space(bool pressed)
	{
		Button *button = held ? stops[*held].button : nullptr;

		if (!button || !button->is_active() || button->get_state() == Button::State::DISABLED)
		{
			spacing = false;

			return std::nullopt;
		}

		if (pressed)
		{
			spacing = true;
			button->set_state(Button::State::PRESSED);

			return std::nullopt;
		}

		if (!spacing)
			return std::nullopt;

		spacing = false;
		button->set_state(Button::State::NORMAL);

		return stops[*held].id;
	}

	bool FocusRing::holds_keys() const
	{
		return held.has_value();
	}

	std::optional<size_t> FocusRing::current() const
	{
		for (size_t i = 0; i < stops.size(); i++)
			if (stops[i].field && stops[i].field->get_state() == Textfield::State::FOCUSED)
				return i;

		return held;
	}

	bool FocusRing::accepts(size_t index) const
	{
		const Stop &stop = stops[index];

		if (stop.field)
			return stop.field->get_state() != Textfield::State::DISABLED;

		if (stop.button)
			return stop.button->is_active() && stop.button->get_state() != Button::State::DISABLED;

		return true;
	}

	void FocusRing::focus(size_t index)
	{
		let_go();

		const Stop &stop = stops[index];

		if (stop.field)
		{
			stop.field->set_state(Textfield::State::FOCUSED);

			return;
		}

		for (const Stop &other : stops)
			if (other.field && other.field->get_state() == Textfield::State::FOCUSED)
				UI::get().remove_textfield();

		held = index;

		if (stop.button)
			stop.button->set_focused(true);
	}

	void FocusRing::let_go()
	{
		if (!held)
			return;

		if (Button *button = stops[*held].button)
		{
			button->set_focused(false);

			if (spacing && button->get_state() == Button::State::PRESSED)
				button->set_state(Button::State::NORMAL);
		}

		held.reset();
		spacing = false;
	}
}
