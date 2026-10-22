#include "UITransform.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// FUN_00a5312b: OK and Cancel, and the edit control, 200 by 14 at (23, 54), taking 12 characters.
		constexpr Point<int16_t> OK_BUTTON = Point<int16_t>(60, 100);
		constexpr Point<int16_t> CANCEL_BUTTON = Point<int16_t>(170, 100);
		constexpr int16_t FIELD_LEFT = 23;
		constexpr int16_t FIELD_TOP = 54;
		constexpr int16_t FIELD_WIDTH = 200;
		constexpr int16_t FIELD_HEIGHT = 14;
		constexpr size_t MAX_NAME = 12;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
	}

	UITransform::UITransform(std::function<bool(const std::string &)> os)
			: UIElement(Point<int16_t>(), Point<int16_t>()), onsend(std::move(os))
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Transform"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOk"], OK_BUTTON);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], CANCEL_BUTTON);

		Rectangle<int16_t> bounds(FIELD_LEFT, FIELD_LEFT + FIELD_WIDTH, FIELD_TOP, FIELD_TOP + FIELD_HEIGHT);
		field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bounds, MAX_NAME);
		field.set_enter_callback(
				[&](std::string)
				{
					send();
				}
		);
		field.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		// Centred on the screen (FUN_00a52f31 makes it 267 by 129 at (266, 235)).
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		field.set_state(Textfield::State::FOCUSED);
	}

	void UITransform::draw(float alpha) const
	{
		draw_sprites(alpha);

		field.draw(position - Point<int16_t>(0, TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UITransform::update()
	{
		UIElement::update();

		field.update(position);
	}

	Cursor::State UITransform::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = field.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UITransform::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			send();
	}

	UIElement::Type UITransform::get_type() const
	{
		return TYPE;
	}

	Button::State UITransform::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				send();
				break;
			case Buttons::CANCEL:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	// FUN_00a5337f: an empty name sends nothing, and the dialog stays.
	void UITransform::send()
	{
		std::string name = field.get_text();

		if (name.empty() || !onsend(name))
			return;

		close();
	}

	void UITransform::close()
	{
		field.set_state(Textfield::State::DISABLED);
		deactivate();
	}
}
