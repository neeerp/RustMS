#include "UIMemoSend.h"
#include "NpcInput.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/Memos.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/InventoryPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		const Point<int16_t> OK_AT(193, 141), CANCEL_AT(242, 141), CLOSE_AT(277, 6);
		// v83's edit boxes are 13 px high, at (38, 67) and (13, 114); our
		// text sits 3 px lower than v83's.
		const Rectangle<int16_t> TO_BOX(38, 188, 64, 77);
		const Rectangle<int16_t> TEXT_BOX(13, 284, 111, 124);

		constexpr const char* NO_MEMO = "You can use it after purchasing the Memo.";
		constexpr const char* REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";
	}

	UIMemoSend::UIMemoSend(int16_t slot, int32_t item_id, const std::string& to)
		: UIElement(Point<int16_t>(), Point<int16_t>()), slot(slot), item_id(item_id)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		background = NxFiles::ui_classic["UIWindow.img"]["Memo"]["backgrnd3"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
				(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		buttons[CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], CANCEL_AT);
		buttons[CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_AT);

		receiver = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, TO_BOX, Memos::LONGEST_NAME);
		message = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, TEXT_BOX, Memos::LONGEST_TEXT);

		for (Textfield* field : { &receiver, &message })
		{
			field->set_enter_callback([&](std::string) { send(); });
			field->set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		}

		receiver.set_key_callback(KeyAction::Id::TAB, [&]()
		{
			receiver.set_state(Textfield::State::NORMAL);
			message.set_state(Textfield::State::FOCUSED);
		});
		message.set_key_callback(KeyAction::Id::TAB, [&]()
		{
			message.set_state(Textfield::State::NORMAL);
			receiver.set_state(Textfield::State::FOCUSED);
		});

		receiver.change_text(to);
		(to.empty() ? receiver : message).set_state(Textfield::State::FOCUSED);
	}

	void UIMemoSend::write_to(const std::string& to)
	{
		int16_t slot = Stage::get().get_player().get_inventory().find_item(InventoryType::Id::CASH, Memos::ITEM);

		if (slot <= 0)
		{
			UI::get().emplace<UIOk>(NO_MEMO, [](bool) {});

			return;
		}

		UI::get().emplace<UIMemoSend>(slot, Memos::ITEM, to);
	}

	void UIMemoSend::draw(float inter) const
	{
		background.draw(position);
		receiver.draw(position);
		message.draw(position);

		UIElement::draw(inter);
	}

	void UIMemoSend::update()
	{
		UIElement::update();

		receiver.update(position);
		message.update(position);
	}

	Cursor::State UIMemoSend::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		for (Textfield* field : { &receiver, &message })
		{
			Cursor::State state = field->send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMemoSend::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			send();
	}

	Button::State UIMemoSend::button_pressed(uint16_t buttonid)
	{
		if (buttonid == OK)
			send();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIMemoSend::send()
	{
		std::string to = npc_input::trim(receiver.get_text());
		std::string text = npc_input::trim(message.get_text());
		Player& player = Stage::get().get_player();

		Memos::Refusal refusal = Memos::check(to, text, player.get_stats().get_name(), CurseData::filter);

		if (refusal != Memos::Refusal::NONE)
		{
			UI::get().emplace<UIOk>(Memos::refusal_text(refusal), [](bool) {});

			return;
		}

		if (!player.request(SendNotePacket(slot, item_id, to, text), ExclRequest::WAIT))
		{
			UI::get().emplace<UIOk>(REQUEST_BUSY, [](bool) {});

			return;
		}

		Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
		close();
	}

	void UIMemoSend::close()
	{
		receiver.set_state(Textfield::State::DISABLED);
		message.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	UIElement::Type UIMemoSend::get_type() const
	{
		return TYPE;
	}
}
