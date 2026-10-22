#include "UIMapleTv.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (OnCreate asm 7f02ba). The box takes
		// 135 px rows wrapped at 125 beside the art's grey margins, but a
		// Star's 180 wrapped at 170 over them.
		constexpr Rectangle<int16_t> TEXT = Rectangle<int16_t>(40, 175, 113, 188);
		constexpr Rectangle<int16_t> STAR_TEXT = Rectangle<int16_t>(18, 198, 113, 188);
		constexpr uint16_t TEXT_WRAP = 125;
		constexpr uint16_t STAR_TEXT_WRAP = 170;
		// The art's rows are 15 px apart, A13M's lines 18.
		constexpr int16_t TEXT_ROW_ADJ = 3;
		// The receiver's edit in the "To" bar, and the bar.
		constexpr Rectangle<int16_t> RECEIVER = Rectangle<int16_t>(44, 194, 68, 82);
		constexpr Point<int16_t> TO_BAR = Point<int16_t>(20, 65);
		// A character's name.
		constexpr size_t NAME_LENGTH = 12;
		// More than five rows hold: the rows are what limits the box, where a
		// limit of 0 would stop it at its width.
		constexpr size_t TEXT_TYPED = 255;
		// The checkbox controls, the Messenger's beside the bar and the
		// ear over "Display whisper icon"; the box is drawn 2 right and 1
		// down in them.
		constexpr Rectangle<int16_t> ADDRESSED_CHECK = Rectangle<int16_t>(5, 19, 65, 79);
		constexpr Rectangle<int16_t> EAR_CHECK = Rectangle<int16_t>(5, 165, 195, 209);
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);
		// v83 draws a line's cap top about 3 below where it is drawn, we 9:
		// both edits are drawn higher by the difference.
		constexpr int16_t TEXT_RAISE = 6;
	}

	UIMapleTv::UIMapleTv(int32_t item_id, OnSend os)
			: UIElement(Point<int16_t>(), Point<int16_t>()), mode(MapleTv::mode(item_id)),
			  mega(MapleTv::mega(item_id)), onsend(std::move(os)), addressed(mode == MapleTv::Mode::HEART),
			  ear(true), held(false), checking(Check::NONE)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MapleTV"];

		// A Star's art has no margins, and a Megassenger's room for the ear.
		bool star = mode == MapleTv::Mode::STAR;
		nl::node background = src[mega ? (star ? "backgrnd4" : "backgrnd3") : (star ? "backgrnd2" : "backgrnd")];
		sprites.emplace_back(background);

		checkbox[false] = basic["CheckBox"]["0"];
		checkbox[true] = basic["CheckBox"]["1"];
		to_bar[false] = src["BtTo"]["disabled"]["0"];
		to_bar[true] = src["BtTo"]["enabled"]["0"];

		int16_t buttons_y = mega ? 218 : 208;
		buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOk"], Point<int16_t>(60, buttons_y));
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(110, buttons_y));

		// The box takes no more than its five rows.
		text = Textfield(Text::Font::A13M, Text::Alignment::LEFT, Color::Name::BLACK, star ? STAR_TEXT : TEXT, TEXT_TYPED);
		text.set_wrap(star ? STAR_TEXT_WRAP : TEXT_WRAP, TEXT_ROW_ADJ, MapleTv::ROWS);

		receiver = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, RECEIVER, NAME_LENGTH);

		for (Textfield *field : { &text, &receiver })
		{
			field->set_key_callback(
					KeyAction::Id::ESCAPE,
					[&]()
					{
						close();
					}
			);
			// Tab moves between the receiver and the box, while there is a
			// receiver to name.
			field->set_key_callback(
					KeyAction::Id::TAB,
					[&, field]()
					{
						Textfield &other = field == &text ? receiver : text;

						if (other.get_state() != Textfield::State::DISABLED)
							other.set_state(Textfield::State::FOCUSED);
					}
			);
		}

		if (!addressed)
			receiver.set_state(Textfield::State::DISABLED);

		// Centred, with no title bar to drag it by.
		dimension = Texture(background).get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		(addressed ? receiver : text).set_state(Textfield::State::FOCUSED);
	}

	void UIMapleTv::draw(float alpha) const
	{
		draw_sprites(alpha);

		// A Star names no one.
		if (mode != MapleTv::Mode::STAR)
			to_bar[addressed].draw(position + TO_BAR);

		if (mode == MapleTv::Mode::MESSENGER)
			checkbox[addressed].draw(position + ADDRESSED_CHECK.get_left_top() + CHECK_BOX);

		if (mega)
			checkbox[ear].draw(position + EAR_CHECK.get_left_top() + CHECK_BOX);

		if (mode != MapleTv::Mode::STAR)
			receiver.draw(position - Point<int16_t>(0, TEXT_RAISE));

		text.draw(position - Point<int16_t>(0, TEXT_RAISE));

		UIElement::draw_buttons(alpha);
	}

	void UIMapleTv::update()
	{
		UIElement::update();

		text.update(position);
		receiver.update(position);
	}

	void UIMapleTv::remove_cursor()
	{
		UIElement::remove_cursor();

		held = false;
		checking = Check::NONE;
	}

	Cursor::State UIMapleTv::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool press = clicked && !held;
		bool release = !clicked && held;
		held = clicked;

		// A checkbox toggles when the button comes up, wherever it is, after going down on it.
		if (release && checking != Check::NONE)
		{
			if (checking == Check::ADDRESSED)
				set_addressed(!addressed);
			else
				ear = !ear;

			checking = Check::NONE;
		}

		for (Textfield *field : { &text, &receiver })
		{
			if (field == &receiver && mode == MapleTv::Mode::STAR)
				continue;

			Cursor::State state = field->send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		Point<int16_t> cursor = cursorpos - position;

		for (Check check : { Check::ADDRESSED, Check::EAR })
		{
			bool shown = check == Check::ADDRESSED ? mode == MapleTv::Mode::MESSENGER : mega;

			if (!shown || !(check == Check::ADDRESSED ? ADDRESSED_CHECK : EAR_CHECK).contains(cursor))
				continue;

			if (press)
				checking = check;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMapleTv::send_key(int32_t, bool pressed, bool escape)
	{
		// Escape is Cancel. Enter does nothing (asm 7f0d69): only OK sends.
		if (pressed && escape)
			close();
	}

	UIElement::Type UIMapleTv::get_type() const
	{
		return TYPE;
	}

	Button::State UIMapleTv::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::OK:
				send();
				break;
			case Buttons::CANCEL:
				close();
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIMapleTv::send()
	{
		std::string name = mode == MapleTv::Mode::STAR ? "" : receiver.get_text();
		MapleTv::Lines lines = MapleTv::lines(text.get_text(), text.line_starts());
		bool whisper = mega && ear;

		close();
		onsend(name, lines, whisper);
	}

	void UIMapleTv::close()
	{
		for (Textfield *field : { &text, &receiver })
			if (field->get_state() == Textfield::State::FOCUSED)
				UI::get().remove_textfield();

		deactivate();
	}

	void UIMapleTv::set_addressed(bool to)
	{
		// Unticked, the receiver's edit is emptied and taken away (asm 7f0cdf).
		addressed = to;

		if (addressed)
		{
			receiver.set_state(Textfield::State::FOCUSED);
		} else
		{
			receiver.change_text("");
			receiver.set_state(Textfield::State::DISABLED);
		}
	}
}
