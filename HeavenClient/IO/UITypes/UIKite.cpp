#include "UIKite.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/MapleMap/KiteRules.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left. v83 draws a line's cap top about 3 below where it is drawn, we 9, so text goes
		// up by the difference.
		constexpr int16_t TEXT_RAISE = 6;
		// The entry window (FUN_007dbcbb, FUN_007dbfa9): the prompt, the three edit controls 366 by 17 at x 28,
		// and OK and Cancel. UIClassic's backgrnd2 has a 50 px header band over its icon and lined rows that the
		// exe's positions (prompt at y 19, fields at 45, 70 and 94) don't allow for, so the prompt and fields go
		// down by that much to sit beside the icon and on the rows; the buttons fit the art as they are.
		constexpr int16_t ART_SHIFT = 50;
		constexpr Point<int16_t> PROMPT = Point<int16_t>(40, 19 + ART_SHIFT);
		constexpr int16_t FIELD_LEFT = 28;
		constexpr int16_t FIELD_WIDTH = 366;
		constexpr int16_t FIELD_HEIGHT = 17;
		constexpr int16_t FIELD_TOPS[] = { 45 + ART_SHIFT, 70 + ART_SHIFT, 94 + ART_SHIFT };
		constexpr Point<int16_t> OK_BUTTON = Point<int16_t>(330, 192);
		constexpr Point<int16_t> CANCEL_BUTTON = Point<int16_t>(372, 192);
		// The viewer (FUN_00659208, FUN_00659300): the name, the item's msg 5 past the name's end, the lines 20
		// apart, and the close button.
		constexpr Point<int16_t> NAME = Point<int16_t>(30, 13);
		constexpr int16_t MSG_GAP = 5;
		constexpr Point<int16_t> FIRST_LINE = Point<int16_t>(22, 47);
		constexpr int16_t LINE_PITCH = 20;
		constexpr Point<int16_t> CLOSE_BUTTON = Point<int16_t>(385, 10);

		nl::node multi_line()
		{
			return NxFiles::ui_classic["UIWindow.img"]["MultiLine"];
		}

		// Centred on the screen (FUN_004edaef), with no title bar to drag it by.
		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
		}

		std::string item_string(int32_t item_id, const char *key)
		{
			return nl::nx::string["Cash.img"][std::to_string(item_id)][key];
		}
	}

	UIKiteWrite::UIKiteWrite(int32_t item_id, std::function<void(const std::vector<std::string> &)> os)
			: UIElement(Point<int16_t>(), Point<int16_t>()), onsend(std::move(os)),
			  prompt(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, item_string(item_id, "msgIn"))
	{
		nl::node src = multi_line();
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		Texture background = src["backgrnd2"];
		sprites.emplace_back(src["backgrnd2"]);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_BUTTON);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_BUTTON);

		for (size_t i = 0; i < LINES; i++)
		{
			Rectangle<int16_t> bounds(FIELD_LEFT, FIELD_LEFT + FIELD_WIDTH, FIELD_TOPS[i], FIELD_TOPS[i] + FIELD_HEIGHT);

			Textfield &field = fields[i];
			field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bounds, kite::MAX_LINE);
			field.set_key_callback(
					KeyAction::Id::ESCAPE,
					[&]()
					{
						close();
					}
			);
			// FUN_004edd21 moves through every control, buttons too; here Tab moves through the lines.
			field.set_key_callback(
					KeyAction::Id::TAB,
					[&, i]()
					{
						fields[(i + 1) % LINES].set_state(Textfield::State::FOCUSED);
					}
			);
		}

		dimension = background.get_dimensions();
		position = centred(dimension);

		fields[0].set_state(Textfield::State::FOCUSED);
	}

	void UIKiteWrite::draw(float alpha) const
	{
		draw_sprites(alpha);

		prompt.draw(position + PROMPT - Point<int16_t>(0, TEXT_RAISE));

		for (const Textfield &field : fields)
			field.draw(position - Point<int16_t>(0, TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UIKiteWrite::update()
	{
		UIElement::update();

		for (Textfield &field : fields)
			field.update(position);
	}

	Cursor::State UIKiteWrite::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		for (Textfield &field : fields)
		{
			Cursor::State state = field.send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIKiteWrite::send_key(int32_t, bool pressed, bool escape)
	{
		// FUN_007dc10f: Enter is swallowed, so only Escape is heard.
		if (pressed && escape)
			close();
	}

	UIElement::Type UIKiteWrite::get_type() const
	{
		return TYPE;
	}

	Button::State UIKiteWrite::button_pressed(uint16_t buttonid)
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

	void UIKiteWrite::send()
	{
		std::vector<std::string> typed;

		for (const Textfield &field : fields)
			typed.push_back(field.get_text());

		close();
		onsend(typed);
	}

	void UIKiteWrite::close()
	{
		for (Textfield &field : fields)
			field.set_state(Textfield::State::DISABLED);

		deactivate();
	}

	UIKiteView::UIKiteView(int32_t item_id, const std::string &flier, const std::string &message)
			: UIElement(Point<int16_t>(), Point<int16_t>()),
			  name(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::PIZAZZ, flier),
			  item_msg(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::KITEMSG, item_string(item_id, "msg"))
	{
		nl::node src = multi_line();

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(src["BtClose"], CLOSE_BUTTON);

		for (const std::string &line : kite::split(message))
			lines.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, line);

		dimension = background.get_dimensions();
		position = centred(dimension);
	}

	void UIKiteView::draw(float alpha) const
	{
		draw_sprites(alpha);

		Point<int16_t> raise = Point<int16_t>(0, TEXT_RAISE);

		name.draw(position + NAME - raise);
		item_msg.draw(position + NAME + Point<int16_t>(name.width() + MSG_GAP, 0) - raise);

		for (size_t i = 0; i < lines.size(); i++)
			lines[i].draw(position + FIRST_LINE + Point<int16_t>(0, static_cast<int16_t>(LINE_PITCH * i)) - raise);

		UIElement::draw_buttons(alpha);
	}

	void UIKiteView::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// FUN_004edd21: Escape and Enter both close it.
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			deactivate();
	}

	UIElement::Type UIKiteView::get_type() const
	{
		return TYPE;
	}

	Button::State UIKiteView::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::CLOSE)
			deactivate();

		return Button::State::NORMAL;
	}
}
