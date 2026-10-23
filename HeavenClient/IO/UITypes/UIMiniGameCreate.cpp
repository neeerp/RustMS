#include "UIMiniGameCreate.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/MapleComboBox.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/MapleMap/FieldLimit.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Handlers/MiniGame.h"
#include "../../Net/Handlers/MiniRoom.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The dialogs' controls: Omok's (0x7dc8ef, b100 §3.1) and match
		// cards', a little lower for the card count (0x7dd5e4, §3.2).
		struct Layout
		{
			Point<int16_t> ok;
			Point<int16_t> cancel;
			Point<int16_t> lock;
			Point<int16_t> checkbox;
			Point<int16_t> password_box;
			Point<int16_t> password_label;
			Rectangle<int16_t> title;
			Rectangle<int16_t> password;
		};

		const Layout OMOK_LAYOUT = {
			Point<int16_t>(156, 152),
			Point<int16_t>(197, 152),
			Point<int16_t>(15, 94),
			Point<int16_t>(60, 94),
			Point<int16_t>(113, 114),
			Point<int16_t>(60, 117),
			Rectangle<int16_t>(25, 244, 50, 65),
			Rectangle<int16_t>(115, 245, 115, 130)
		};

		const Layout CARDS_LAYOUT = {
			Point<int16_t>(176, 180),
			Point<int16_t>(218, 180),
			Point<int16_t>(15, 121),
			Point<int16_t>(61, 121),
			Point<int16_t>(113, 141),
			Point<int16_t>(61, 144),
			Rectangle<int16_t>(25, 243, 50, 65),
			Rectangle<int16_t>(115, 245, 142, 157)
		};

		// The card count, the board's size by index (b100 §3.2).
		constexpr Point<int16_t> CARD_COUNT_AT = Point<int16_t>(160, 83);
		constexpr int64_t CARD_COUNT_WIDTH = 85;

		const Layout &layout_of(int32_t itemid)
		{
			return mini_game::is_omok_set(itemid) ? OMOK_LAYOUT : CARDS_LAYOUT;
		}

		// v83 draws a line's cap top about 3 below where it is drawn, we 9.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);
	}

	UIMiniGameCreate::UIMiniGameCreate(int32_t itemid) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)),
			itemid(itemid), omok(mini_game::is_omok_set(itemid)), locked(false)
	{
		nl::node Minigame = NxFiles::ui_classic["UIWindow.img"]["Minigame"];
		nl::node Common = Minigame["Common"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		const Layout &layout = layout_of(itemid);

		backgrnd = Minigame[omok ? "Omok" : "MemoryGame"]["backgrnd2"];
		lock[false] = Common["Unlock"];
		lock[true] = Common["Lock"];
		checkbox[false] = Basic["CheckBox"]["0"];
		checkbox[true] = Basic["CheckBox"]["1"];
		password_box[false] = Common["text"]["disabled"];
		password_box[true] = Common["text"]["normal"];
		password_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Password");

		// Centred, as v83's dialogs are (0x4edaef).
		dimension = backgrnd.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
				(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Basic["BtOK2"], layout.ok);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], layout.cancel);

		if (!omok)
			buttons[Buttons::CARD_COUNT] = std::make_unique<MapleComboBox>(
				MapleComboBox::Type::DEFAULT, std::vector<std::string> { "4 X 3", "5 X 4", "6 X 5" }, 0, position,
				CARD_COUNT_AT, CARD_COUNT_WIDTH
			);

		title = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, layout.title, mini_game::TITLE_LIMIT);
		title.set_enter_callback([&](std::string) { confirm(); });
		title.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		title.set_state(Textfield::State::FOCUSED);

		password = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, layout.password, mini_game::PASSWORD_LIMIT);
		password.set_enter_callback([&](std::string) { confirm(); });
		password.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		password.set_state(Textfield::State::DISABLED);

		buttons[Buttons::OK]->set_state(Button::State::DISABLED);
	}

	void UIMiniGameCreate::draw(float inter) const
	{
		const Layout &layout = layout_of(itemid);

		backgrnd.draw(position);
		lock[locked].draw(position + layout.lock);
		checkbox[locked].draw(position + layout.checkbox);
		password_box[locked].draw(position + layout.password_box);
		password_label.draw(position + layout.password_label + TEXT_Y);

		title.draw(position);
		password.draw(position);

		UIElement::draw(inter);
	}

	void UIMiniGameCreate::update()
	{
		UIElement::update();

		title.update(position);
		password.update(position);

		// OK waits for a title, and a password if locked (0x7dd440). Only a
		// change is set, so a hover or press on it lasts.
		auto &ok = buttons[Buttons::OK];
		bool enabled = can_confirm();

		if (enabled != (ok->get_state() != Button::State::DISABLED))
			ok->set_state(enabled ? Button::State::NORMAL : Button::State::DISABLED);
	}

	Cursor::State UIMiniGameCreate::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// The open list covers what is under it.
		if (!omok)
		{
			auto &combo = buttons[Buttons::CARD_COUNT];

			if (combo->is_pressed() && combo->in_combobox(cursorpos))
			{
				Cursor::State state = combo->send_cursor(clicked, cursorpos);

				return clicked && state == Cursor::State::IDLE ? Cursor::State::CLICKING : state;
			}
		}

		Point<int16_t> at = cursorpos - position - layout_of(itemid).checkbox;
		Point<int16_t> size = checkbox[locked].get_dimensions();

		if (at.x() >= 0 && at.y() >= 0 && at.x() < size.x() && at.y() < size.y())
		{
			if (clicked)
			{
				toggle_lock();

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		if (Cursor::State state = title.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
			return state;

		if (locked)
			if (Cursor::State state = password.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
				return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMiniGameCreate::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UIMiniGameCreate::get_type() const
	{
		return TYPE;
	}

	void UIMiniGameCreate::open(int32_t itemid)
	{
		if (!mini_game::is_game_set(itemid))
			return;

		Player &player = Stage::get().get_player();

		if (player.ride_refuses() || player.shape_refuses() || player.ghost_refuses())
			return;

		if (UI::get().dialog_open())
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("Not available", UIChatBar::LineType::SKILL);

			return;
		}

		if (Stage::get().get_mapinfo().get_fieldlimit() & field_limit::MINI_GAMES)
		{
			UI::get().emplace<UIOk>("You can't start the game here.", [](bool) {});

			return;
		}

		UI::get().emplace<UIMiniGameCreate>(itemid);
	}

	Button::State UIMiniGameCreate::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::CARD_COUNT)
		{
			buttons[Buttons::CARD_COUNT]->toggle_pressed();

			return Button::State::NORMAL;
		}

		if (buttonid == Buttons::OK)
			confirm();
		else
			close();

		return Button::State::NORMAL;
	}

	void UIMiniGameCreate::toggle_lock()
	{
		// Unlocking clears the password (0x7dd255).
		locked = !locked;

		if (locked)
		{
			title.set_state(Textfield::State::NORMAL);
			password.set_state(Textfield::State::FOCUSED);
		}
		else
		{
			password.change_text("");
			password.set_state(Textfield::State::DISABLED);
			title.set_state(Textfield::State::FOCUSED);
		}
	}

	void UIMiniGameCreate::confirm()
	{
		if (!can_confirm())
			return;

		std::string text = title.get_text();
		std::string key = locked ? password.get_text() : "";
		uint16_t size = omok ? 0 : buttons[Buttons::CARD_COUNT]->get_selected();

		close();

		// A curse word is starred out; text the filter won't take is refused
		// (0x7a06f3).
		if (!CurseData::valid(text))
		{
			UI::get().emplace<UIOk>("Cursing is not allowed.", [](bool) {});

			return;
		}

		// Omok plays with the set's stones, match cards on the chosen board.
		if (omok)
			GameCreatePacket(mini_room::OMOK, CurseData::filter(text), key, static_cast<int8_t>(itemid % 100)).dispatch();
		else
			GameCreatePacket(mini_room::MATCH_CARDS, CurseData::filter(text), key, static_cast<int8_t>(size)).dispatch();
	}

	void UIMiniGameCreate::close()
	{
		title.set_state(Textfield::State::DISABLED);
		password.set_state(Textfield::State::DISABLED);

		deactivate();
	}

	bool UIMiniGameCreate::can_confirm() const
	{
		return !title.empty() && (!locked || !password.empty());
	}
}
