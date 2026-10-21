#include "UIMegaphone.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Megaphones.h"
#include "../../Util/Misc.h"
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
		// The avatar megaphone's box (FUN_007f1196): 107 by 60 at (48, 81).
		constexpr Rectangle<int16_t> AVATAR_FIELD = Rectangle<int16_t>(48, 155, 81, 141);
		// The checkbox control; its box is drawn 2 right and 1 down in it.
		constexpr Rectangle<int16_t> SUPER_CHECK = Rectangle<int16_t>(14, 174, 95, 109);
		constexpr Rectangle<int16_t> TRIPLE_CHECK = Rectangle<int16_t>(18, 178, 153, 167);
		constexpr Rectangle<int16_t> AVATAR_CHECK = Rectangle<int16_t>(10, 170, 147, 161);
		constexpr Point<int16_t> CHECK_BOX = Point<int16_t>(2, 1);
		// The samples' popup (FUN_00999320 and FUN_009995ae): where it opens from the window's top left, its width,
		// its rows, the first sample button in it, and the "CREATE NEW" bar that empties the lines.
		constexpr Point<int16_t> SAMPLES = Point<int16_t>(407, 30);
		constexpr int16_t SAMPLES_WIDTH = 100;
		constexpr int16_t SAMPLE_ROW = 15;
		constexpr Point<int16_t> SAMPLES_ROWS = Point<int16_t>(3, 23);
		constexpr Point<int16_t> SAMPLE_BUTTON = Point<int16_t>(9, 24);
		constexpr Rectangle<int16_t> SAMPLES_CLEAR = Rectangle<int16_t>(11, 76, 10, 21);

		// StringPool 0x1172 and 0x1173.
		constexpr const char *PREVIEW_HEADER = "This is a preview option for the Triple Megaphone.";
		constexpr const char *LINE_MISSING = "One of the lines has not been applied.\\nWould you still like to use it?";
	}

	UIMegaphone::UIMegaphone(int32_t item_id, std::function<void(const std::vector<std::string> &, bool)> os)
			: UIElement(Point<int16_t>(), Point<int16_t>()),
			  triple(Megaphones::kind(item_id) == Megaphones::Kind::TRIPLE),
			  avatar(Megaphones::kind(item_id) == Megaphones::Kind::AVATAR),
			  item(nl::nx::item["Cash"][string_format::extend_id(item_id / 10'000, 4) + ".img"]
				   [string_format::extend_id(item_id, 8)]), onsend(std::move(os)),
			  ear(true), held(false), checking(false), sampling(false), sample_count(0)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"][triple ? "ArtMegaphone" : avatar ? "AvatarMegaphone" :
																					   "Megaphone"];

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
		} else if (avatar)
		{
			buttons[Buttons::OK] = std::make_unique<MapleButton>(src["BtOk"], Point<int16_t>(52, 170));
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(src["BtCancel"], Point<int16_t>(102, 170));

			field_count = 1;
			check_area = AVATAR_CHECK;
		} else
		{
			buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], Point<int16_t>(146, 126));
			buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], Point<int16_t>(198, 126));

			field_count = 1;
			check_area = SUPER_CHECK;
		}

		for (size_t i = 0; i < field_count; i++)
		{
			Rectangle<int16_t> bounds = triple ? TRIPLE_FIELD : avatar ? AVATAR_FIELD : SUPER_FIELD;
			bounds.shift(Point<int16_t>(0, triple ? TRIPLE_LINES[i] : 0));

			Textfield &field = fields[i];
			field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, bounds,
							  avatar ? Megaphones::AVATAR_TYPED : Megaphones::MAX_LINE);

			if (avatar)
				field.set_wrap(bounds.width());

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

			// FUN_004edd21 moves through every control, buttons too; here Tab moves through the lines.
			if (triple)
				field.set_key_callback(
						KeyAction::Id::TAB,
						[&, i]()
						{
							focus_field(static_cast<int16_t>((i + 1) % field_count));
						}
				);
		}

		// Centred, with no title bar to drag it by.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		if (triple)
		{
			// The item's `ui` holds a button for each sample, and the popup's top, rows and bottom.
			nl::node ui = item["ui"];

			samples_top = ui["t"];
			samples_row = ui["c"];
			samples_bottom = ui["s"];
			sample_count = static_cast<uint16_t>(ui.size() > 3 ? ui.size() - 3 : 0);

			// Kept on the screen.
			int16_t height = (sample_count + 2) * SAMPLE_ROW;
			Point<int16_t> screen = position + SAMPLES;

			if (screen.y() + height > Constants::Constants::get().get_viewheight())
				screen.set_y(Constants::Constants::get().get_viewheight() - height);

			if (screen.x() + SAMPLES_WIDTH > Constants::Constants::get().get_viewwidth())
				screen.set_x(Constants::Constants::get().get_viewwidth() - SAMPLES_WIDTH);

			samples_pos = screen - position;

			for (uint16_t i = 0; i < sample_count; i++)
			{
				auto button = std::make_unique<MapleButton>(
						ui[std::to_string(i)], samples_pos + SAMPLE_BUTTON + Point<int16_t>(0, SAMPLE_ROW * i));
				button->set_active(false);
				buttons[Buttons::SAMPLE + i] = std::move(button);
			}
		}

		fields[0].set_state(Textfield::State::FOCUSED);
	}

	void UIMegaphone::draw(float alpha) const
	{
		draw_sprites(alpha);

		for (auto &iter : buttons)
			if (iter.first < Buttons::SAMPLE)
				iter.second->draw(position);

		for (size_t i = 0; i < field_count; i++)
			fields[i].draw(position - Point<int16_t>(0, TEXT_RAISE));

		checkbox[ear].draw(position + check_area.get_left_top() + CHECK_BOX);

		if (!sampling)
			return;

		Point<int16_t> popup = position + samples_pos;

		samples_top.draw(popup);

		for (uint16_t i = 0; i < sample_count; i++)
			samples_row.draw(popup + SAMPLES_ROWS + Point<int16_t>(0, SAMPLE_ROW * i));

		samples_bottom.draw(popup + SAMPLES_ROWS + Point<int16_t>(0, SAMPLE_ROW * sample_count));

		for (uint16_t i = 0; i < sample_count; i++)
			buttons.at(Buttons::SAMPLE + i)->draw(position);
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
		checking = false;
	}

	Cursor::State UIMegaphone::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool press = clicked && !held;
		bool release = !clicked && held;
		held = clicked;

		if (sampling)
			return send_samples_cursor(press, release, clicked, cursorpos);

		// The checkbox toggles when the button comes up, wherever it is, after going down on it.
		if (release && checking)
		{
			checking = false;
			ear = !ear;
		}

		for (size_t i = 0; i < field_count; i++)
		{
			Cursor::State state = fields[i].send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		if (check_area.contains(cursorpos - position))
		{
			if (press)
				checking = true;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	Cursor::State UIMegaphone::send_samples_cursor(bool press, bool release, bool clicked, Point<int16_t> cursorpos)
	{
		// FUN_00999b5b: the popup has the mouse to itself. The button going down or up outside it closes it, and
		// going down on the "CREATE NEW" bar empties the lines.
		Point<int16_t> cursor = cursorpos - position - samples_pos;
		Rectangle<int16_t> popup = Rectangle<int16_t>(0, SAMPLES_WIDTH - 1, 0, (sample_count + 2) * SAMPLE_ROW - 1);

		if ((press || release) && !popup.contains(cursor))
		{
			UIElement::send_cursor(clicked, cursorpos);
			use_sample(std::nullopt);

			return Cursor::State::IDLE;
		}

		if (press && SAMPLES_CLEAR.contains(cursor))
		{
			for (size_t i = 0; i < field_count; i++)
				fields[i].change_text("");

			use_sample(std::nullopt);

			return Cursor::State::IDLE;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMegaphone::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		// FUN_004edd21: Enter is OK, and Escape is Cancel. Over the samples, Escape closes them, and Enter picks
		// none.
		if (sampling)
		{
			if (escape || keycode == KeyAction::Id::RETURN)
				use_sample(std::nullopt);
		} else if (escape)
		{
			close();
		} else if (keycode == KeyAction::Id::RETURN)
		{
			send();
		}
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
			case Buttons::EXAMPLE:
				open_samples();
				break;
			case Buttons::PREVIEW:
				preview();
				break;
			default:
				if (buttonid >= Buttons::SAMPLE)
					use_sample(static_cast<uint16_t>(buttonid - Buttons::SAMPLE));

				break;
		}

		return Button::State::NORMAL;
	}

	bool UIMegaphone::button_covered(uint16_t buttonid, Point<int16_t>) const
	{
		return sampling && buttonid < Buttons::SAMPLE;
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

	void UIMegaphone::open_samples()
	{
		// FUN_007e0fbf: the samples are modal over the window. Clicking the button took the keys from the lines, and
		// they don't get them back.
		sampling = true;
		checking = false;

		for (uint16_t i = 0; i < sample_count; i++)
		{
			Button &button = *buttons[Buttons::SAMPLE + i];
			button.set_state(Button::State::NORMAL);
			button.set_active(true);
		}
	}

	void UIMegaphone::use_sample(std::optional<uint16_t> index)
	{
		// FUN_007e1052: a sample replaces all three lines as they are, without the typed limit, and a line it
		// lacks is left empty.
		if (index)
		{
			nl::node sample = item["sample"][std::to_string(*index)];

			for (size_t i = 0; i < field_count; i++)
				fields[i].change_text(Megaphones::sample_line(sample[std::to_string(i)].get_string()));
		}

		sampling = false;

		for (uint16_t i = 0; i < sample_count; i++)
			buttons[Buttons::SAMPLE + i]->set_active(false);
	}

	void UIMegaphone::focus_field(int16_t index)
	{
		if (index >= 0)
			fields[index].set_state(Textfield::State::FOCUSED);
	}

	void UIMegaphone::preview()
	{
		std::vector<std::string> shown = Megaphones::lines_sent(lines());

		// FUN_007e0c37: nothing written shows nothing. Otherwise a header, then the lines up to the last one
		// written, only in my own chat log, as they would be heard.
		if (shown.size() == 1 && shown[0].empty())
			return;

		auto chatbar = UI::get().get_element<UIChatBar>();

		if (!chatbar)
			return;

		const std::string &name = Stage::get().get_player().get_stats().get_name();

		chatbar->send_chatline(PREVIEW_HEADER, UIChatBar::LineType::GREY);

		for (const std::string &line : shown)
			chatbar->send_chatline(name + Megaphones::SEPARATOR + line, UIChatBar::LineType::MEGAPHONE_PREVIEW);
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
		if (avatar)
		{
			Megaphones::AvatarLines rows = Megaphones::avatar_lines(fields[0].get_text(), fields[0].line_starts());

			return std::vector<std::string>(rows.begin(), rows.end());
		}

		std::vector<std::string> typed;

		for (size_t i = 0; i < field_count; i++)
			typed.push_back(fields[i].get_text());

		return typed;
	}
}
