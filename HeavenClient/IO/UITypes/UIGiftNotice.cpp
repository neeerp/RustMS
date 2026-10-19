#include "UIGiftNotice.h"
#include "NpcInput.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 draws its text about 6 pixels higher than ours.
		constexpr int16_t TEXT_RAISE = 6;
		// The item's name and the sender are centred over the blue box.
		constexpr int16_t COLUMN_X = 103;
		constexpr int16_t COLUMN_WIDTH = 134;
		constexpr int16_t NAME_Y = 26;
		constexpr int16_t SENDER_Y = 53;
		const Point<int16_t> SENT_YOU_AT = Point<int16_t>(107, 71);
		const Point<int16_t> LINE1_AT = Point<int16_t>(25, 105);
		const Point<int16_t> LINE2_AT = Point<int16_t>(25, 121);
		constexpr int16_t LINE_WIDTH = 210;
		// The icon's origin; it is drawn doubled to fill the white box.
		const Point<int16_t> ICON_AT = Point<int16_t>(26, 88);
		const Rectangle<int16_t> NOTE_BOX = Rectangle<int16_t>(25, 235, 174 - TEXT_RAISE, 187);
		constexpr size_t NOTE_LIMIT = 200;
		const Point<int16_t> OK_AT = Point<int16_t>(203, 220);

		int16_t centred(const Text& text)
		{
			return COLUMN_X + (COLUMN_WIDTH - text.width()) / 2;
		}
	}

	UIGiftNotice::UIGiftNotice(std::vector<CashGift> presents, size_t at) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), gifts(std::move(presents)), index(at)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["3"]["backgrnd"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"], OK_AT);

		sent_you = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "sent you a present.");

		note = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NOTE_BOX, NOTE_LIMIT);
		note.set_enter_callback([&](std::string) { send_note(); });
		note.set_key_callback(KeyAction::Id::ESCAPE, [&]() { skip(); });

		show(index);
	}

	void UIGiftNotice::show(size_t at)
	{
		index = at;

		const CashGift& gift = gifts[index];

		name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(gift.item_id).get_name());
		string_format::format_with_ellipsis(name, COLUMN_WIDTH);
		sender = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::GIFTBLUE, gift.sender);

		// A second line, if any, follows the first line break.
		size_t brk = gift.message.find('\n');
		line1 = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, gift.message.substr(0, brk));
		line2 = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, brk == std::string::npos ? "" : gift.message.substr(brk + 1));
		string_format::format_with_ellipsis(line1, LINE_WIDTH);
		string_format::format_with_ellipsis(line2, LINE_WIDTH);

		note.change_text("");
		note.set_state(Textfield::State::FOCUSED);
	}

	void UIGiftNotice::draw(float inter) const
	{
		Point<int16_t> raise(0, TEXT_RAISE);

		background.draw(position);
		ItemData::get(gifts[index].item_id).get_icon(false).draw(DrawArgument(position + ICON_AT, 2.0f, 2.0f));

		name.draw(position + Point<int16_t>(centred(name), NAME_Y) - raise);
		sender.draw(position + Point<int16_t>(centred(sender), SENDER_Y) - raise);
		sent_you.draw(position + SENT_YOU_AT - raise);
		line1.draw(position + LINE1_AT - raise);
		line2.draw(position + LINE2_AT - raise);

		note.draw(position);

		UIElement::draw(inter);
	}

	void UIGiftNotice::update()
	{
		UIElement::update();

		note.update(position);
	}

	Cursor::State UIGiftNotice::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = note.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGiftNotice::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			skip();
		else if (keycode == KeyAction::Id::RETURN)
			send_note();
	}

	UIElement::Type UIGiftNotice::get_type() const
	{
		return TYPE;
	}

	Button::State UIGiftNotice::button_pressed(uint16_t buttonid)
	{
		send_note();

		return Button::State::NORMAL;
	}

	void UIGiftNotice::send_note()
	{
		// The note goes to whoever sent the present, without its curses.
		std::string text = CurseData::filter(npc_input::trim(note.get_text()));
		const CashGift& gift = gifts[index];

		GiftNotePacket(gift.sender, text, static_cast<int32_t>(index), gift.cash_id).dispatch();

		note.set_state(Textfield::State::DISABLED);
		deactivate();

		// Sent or not, the note is said to be, and then comes the next present.
		std::vector<CashGift> rest = gifts;
		size_t next = index + 1;

		UI::get().emplace<UIOk>("The note has successfully been sent", [rest, next](bool)
		{
			if (next < rest.size())
				UI::get().emplace<UIGiftNotice>(rest, next);
		});
	}

	void UIGiftNotice::skip()
	{
		if (index + 1 < gifts.size())
		{
			show(index + 1);

			return;
		}

		note.set_state(Textfield::State::DISABLED);
		deactivate();
	}
}
