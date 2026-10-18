#include "UIMegaphone.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Megaphones.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left. v83 draws a line's cap top about 3
		// below where it is drawn, we 9, so the typed text goes up by the
		// difference.
		constexpr int16_t TEXT_RAISE = 6;
		constexpr Rectangle<int16_t> SUPER_FIELD = Rectangle<int16_t>(19, 245, 58, 73);
		constexpr Rectangle<int16_t> TRIPLE_FIELD = Rectangle<int16_t>(19, 414, 63, 79);
		constexpr int16_t TRIPLE_LINES[] = { 0, 21, 43 };
		// The checkbox control; its box is drawn 2 right and 1 down in it.
		constexpr Rectangle<int16_t> SUPER_CHECK = Rectangle<int16_t>(14, 174, 95, 109);
		constexpr Rectangle<int16_t> TRIPLE_CHECK = Rectangle<int16_t>(18, 178, 153, 167);
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);

		// StringPool 0x1172 and 0x1173.
		constexpr const char *PREVIEW_EMPTY = "This is a preview option for the Triple Megaphone.";
		constexpr const char *LINE_MISSING = "One of the lines has not been applied.\\nWould you still like to use it?";
	}

	UIMegaphone::UIMegaphone(bool t, std::function<void(const std::vector<std::string> &, bool)> os)
			: UIElement(Point<int16_t>(), Point<int16_t>()), triple(t), onsend(std::move(os)), ear(true),
			  held(false)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"][triple ? "ArtMegaphone" : "Megaphone"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		checkbox[false] = basic["CheckBox"]["0"];
		checkbox[true] = basic["CheckBox"]["1"];

		if (triple)
		{
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], Point<int16_t>(340, 152));
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], Point<int16_t>(381, 152));
			buttons[Buttons::EXAMPLE] = std::make_unique<MapleButton>(src["BtEx"], Point<int16_t>(350, 33));
			buttons[Buttons::PREVIEW] = std::make_unique<MapleButton>(src["BtPreView"], Point<int16_t>(277, 152));

			field_count = MAX_FIELDS;
			check_area = TRIPLE_CHECK;
		} else
		{
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], Point<int16_t>(146, 126));
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], Point<int16_t>(198, 126));

			field_count = 1;
			check_area = SUPER_CHECK;
		}

		for (size_t i = 0; i < field_count; i++)
		{
			Rectangle<int16_t> bounds = triple ? TRIPLE_FIELD : SUPER_FIELD;
			bounds.shift(Point<int16_t>(0, triple ? TRIPLE_LINES[i] : 0));

			Textfield &field = fields[i];
			field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bounds,
							  Megaphones::MAX_LINE);
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
		}

		// Centred, with no title bar to drag it by.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		fields[0].set_state(Textfield::State::FOCUSED);
	}

	void UIMegaphone::draw(float alpha) const
	{
		UIElement::draw(alpha);

		for (size_t i = 0; i < field_count; i++)
			fields[i].draw(position - Point<int16_t>(0, TEXT_RAISE));

		checkbox[ear].draw(position + check_area.get_left_top() + CHECK_BOX);
	}

	void UIMegaphone::update()
	{
		UIElement::update();

		for (size_t i = 0; i < field_count; i++)
			fields[i].update(position);
	}

	void UIMegaphone::remove_cursor()
	{
		UIElement::remove_cursor();

		held = false;
	}

	Cursor::State UIMegaphone::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// The checkbox answers only to the button going down.
		bool press = clicked && !held;
		held = clicked;

		for (size_t i = 0; i < field_count; i++)
		{
			Cursor::State state = fields[i].send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		if (check_area.contains(cursorpos - position))
		{
			if (press)
				ear = !ear;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMegaphone::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// FUN_004edd21: Enter is OK, and Escape is Cancel.
		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			send();
	}

	UIElement::Type UIMegaphone::get_type() const
	{
		return TYPE;
	}

	Button::State UIMegaphone::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				send();
				break;
			case Buttons::CANCEL:
				close();
				break;
			case Buttons::PREVIEW:
				preview();
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIMegaphone::send()
	{
		std::vector<std::string> typed = lines();

		auto done = [typed](UIMegaphone &window)
		{
			window.close();
			window.onsend(typed, window.ear);
		};

		// FUN_007e1403: the triple megaphone asks before leaving a line out.
		for (const std::string &line : typed)
		{
			if (line.empty() && triple)
			{
				ask(LINE_MISSING, done);

				return;
			}
		}

		done(*this);
	}

	void UIMegaphone::preview()
	{
		std::vector<std::string> typed = lines();

		if (Megaphones::check_lines(typed) == Megaphones::Refusal::QUIET)
		{
			notice(PREVIEW_EMPTY);

			return;
		}

		// FUN_007e0c37: each line goes only to my own chat log, as it would be heard.
		auto chatbar = UI::get().get_element<UIChatBar>();

		if (!chatbar)
			return;

		const std::string &name = Stage::get().get_player().get_stats().get_name();

		for (const std::string &line : typed)
			if (!line.empty())
				chatbar->send_chatline(name + Megaphones::SEPARATOR + line, UIChatBar::LineType::GREY);
	}

	void UIMegaphone::close()
	{
		release_field();
		deactivate();
	}

	void UIMegaphone::notice(const std::string &text)
	{
		int16_t refocus = release_field();

		UI::get().emplace<UIOk>(
				text,
				[refocus](bool)
				{
					after_notice(refocus);
				}
		);
	}

	void UIMegaphone::ask(const std::string &question, std::function<void(UIMegaphone &)> on_yes)
	{
		int16_t refocus = release_field();

		UI::get().emplace<UIYesNo>(
				question,
				[refocus, on_yes](bool yes)
				{
					if (auto window = after_notice(refocus); window && yes)
						on_yes(*window);
				}
		);
	}

	Optional<UIMegaphone> UIMegaphone::after_notice(int16_t refocus)
	{
		auto window = UI::get().get_element<UIMegaphone>();

		if (!window || !window->is_active())
			return {};

		if (refocus >= 0)
			window->fields[refocus].set_state(Textfield::State::FOCUSED);

		return window;
	}

	int16_t UIMegaphone::release_field()
	{
		for (size_t i = 0; i < field_count; i++)
		{
			if (fields[i].get_state() == Textfield::State::FOCUSED)
			{
				UI::get().remove_textfield();

				return static_cast<int16_t>(i);
			}
		}

		return -1;
	}

	std::vector<std::string> UIMegaphone::lines() const
	{
		std::vector<std::string> typed;

		for (size_t i = 0; i < field_count; i++)
			typed.push_back(fields[i].get_text());

		return typed;
	}
}
