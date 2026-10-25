#include "UIGmBlock.h"

#include "../KeyAction.h"

#include "../Components/MapleButton.h"
#include "../Components/MapleComboBox.h"

#include "../../Constants.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/NxFiles.h"

#include <cstdlib>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The controls (b176 §3, §4).
		constexpr Point<int16_t> OK_AT = Point<int16_t>(107, 132);
		constexpr Point<int16_t> CANCEL_AT = Point<int16_t>(156, 132);
		constexpr Point<int16_t> CLOSE_AT = Point<int16_t>(190, 6);
		constexpr Point<int16_t> REASON_AT = Point<int16_t>(103, 68);
		// The combo is 100 wide, its caps 5 and 18.
		constexpr int64_t COMBO_MIDDLE = 77;
		// The name's text top.
		constexpr Point<int16_t> NAME_AT = Point<int16_t>(105, 33);
		constexpr int16_t NAME_WIDTH = 95;
		// The days, 95 by 14 from (105, 51), and the description, 190 by
		// 14 from (10, 105), their text 2 px in.
		constexpr Point<int16_t> DAYS_AT = Point<int16_t>(105, 51);
		constexpr int16_t DAYS_WIDTH = 95;
		constexpr Point<int16_t> DESCRIPTION_AT = Point<int16_t>(10, 105);
		constexpr int16_t DESCRIPTION_WIDTH = 190;
		constexpr int16_t EDIT_HEIGHT = 14;
		constexpr int16_t EDIT_INSET = 2;
		constexpr size_t DAYS_LIMIT = 4;
		constexpr size_t DESCRIPTION_LIMIT = 30;

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line =
					Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "|").height();

			return y + 11 - line;
		}

		// An edit's text area, from the edit's top left.
		Rectangle<int16_t> edit_area(Point<int16_t> at, int16_t width)
		{
			int16_t top = text_y(at.y() + EDIT_INSET);

			return Rectangle<int16_t>(at.x() + EDIT_INSET, at.x() + width - EDIT_INSET, top, top + EDIT_HEIGHT);
		}

		// `text` cut with ".." to `width`, if it is wider.
		std::string fit(const std::string &text, int16_t width)
		{
			Text measure(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);

			if (measure.width() <= width)
				return text;

			std::string cut = text;

			while (!cut.empty())
			{
				cut.pop_back();
				measure.change_text(cut + "..");

				if (measure.width() <= width)
					break;
			}

			return cut + "..";
		}
	}

	UIGmBlock::UIGmBlock(const std::string &n, int32_t d, int32_t reason, const std::vector<std::string> &reasons) :
			UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), name(n)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = NxFiles::ui_classic["UIWindow.img"]["Admin"]["Block"]["backgrnd"];
		dimension = backgrnd.get_dimensions();
		// Centred, as v83's dialogs are (0x4edaef).
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
				(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_AT);
		buttons[Buttons::REASON] = std::make_unique<MapleComboBox>(MapleComboBox::Type::DEFAULT, reasons,
				static_cast<uint16_t>(reason - 1), position, REASON_AT, COMBO_MIDDLE, true);

		// The only text the dialog draws; its labels are in the art.
		name_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, fit(name, NAME_WIDTH));

		// Neither edit checks what is typed (0x9987cd): the days are atoi's.
		days = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, edit_area(DAYS_AT, DAYS_WIDTH),
				DAYS_LIMIT);
		days.change_text(std::to_string(d));
		description = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, edit_area(DESCRIPTION_AT,
				DESCRIPTION_WIDTH), DESCRIPTION_LIMIT);

		// Enter is OK, Esc is Cancel, and Tab goes to the other edit, from
		// wherever the focus is (0x998749).
		for (Textfield *field : { &days, &description })
		{
			field->set_enter_callback([this](std::string) { confirm(); });
			field->set_key_callback(KeyAction::Id::ESCAPE, [this]() { close(); });
			field->set_key_callback(KeyAction::Id::TAB, [this]() { tab(); });
		}

		description.set_state(Textfield::State::FOCUSED);
	}

	void UIGmBlock::draw(float inter) const
	{
		backgrnd.draw(position);
		name_label.draw(position + Point<int16_t>(NAME_AT.x(), text_y(NAME_AT.y())));
		days.draw(position);
		description.draw(position);

		UIElement::draw(inter);
	}

	void UIGmBlock::update()
	{
		UIElement::update();

		days.update(position);
		description.update(position);
	}

	Cursor::State UIGmBlock::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// An open list covers what is under it.
		auto &combo = buttons[Buttons::REASON];

		if (combo->is_pressed() && combo->in_combobox(cursorpos))
		{
			Cursor::State state = combo->send_cursor(clicked, cursorpos);

			return clicked && state == Cursor::State::IDLE ? Cursor::State::CLICKING : state;
		}

		for (Textfield *field : { &days, &description })
			if (Cursor::State state = field->send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
				return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGmBlock::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
		else if (keycode == KeyAction::Id::TAB)
			tab();
	}

	UIElement::Type UIGmBlock::get_type() const
	{
		return TYPE;
	}

	std::vector<std::string> UIGmBlock::reasons()
	{
		// By number, not in NX's order, which puts "10" before "2"
		// (0x998238).
		std::vector<std::string> types;
		nl::node reasons = nl::nx::etc["BlockReason.img"];

		for (int32_t i = 1; reasons[std::to_string(i)]; i++)
			types.push_back(reasons[std::to_string(i)]["type"].get_string());

		return types;
	}

	Button::State UIGmBlock::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::REASON:
				buttons[buttonid]->toggle_pressed();
				break;
			case Buttons::OK:
				confirm();
				break;
			default:
				close();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIGmBlock::confirm()
	{
		// The combo's place, from 1, is the reason's number (0x998848).
		uint8_t reason = static_cast<uint8_t>(buttons[Buttons::REASON]->get_selected() + 1);

		AdminBlockPacket(name, reason, std::atoi(days.get_text().c_str()), description.get_text()).dispatch();

		close();
	}

	void UIGmBlock::tab()
	{
		bool to_description = days.get_state() == Textfield::State::FOCUSED;

		days.set_state(to_description ? Textfield::State::NORMAL : Textfield::State::FOCUSED);
		description.set_state(to_description ? Textfield::State::FOCUSED : Textfield::State::NORMAL);
	}

	void UIGmBlock::close()
	{
		days.set_state(Textfield::State::DISABLED);
		description.set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
