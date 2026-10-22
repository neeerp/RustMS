#include "UITradingRoom.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/Charset.h"
#include "../Components/MapleButton.h"

#include "../../Character/ExclRequest.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/Geometry.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <limits>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using mini_room::Side;

		// The v83 dialog's controls and drawing (FUN_007c23d0, FUN_007c2c3b),
		// by side: mine on the right, the partner's on the left.
		constexpr Point<int16_t> BT_TRADE = Point<int16_t>(116, 49);
		constexpr Point<int16_t> BT_RESET = Point<int16_t>(116, 71);
		constexpr Point<int16_t> BT_COIN = Point<int16_t>(170, 244);
		constexpr Point<int16_t> BT_ENTER = Point<int16_t>(520, 244);
		constexpr Point<int16_t> BT_CLAIM = Point<int16_t>(494, 241);

		// v83 draws a line's cap top about 3 below where it is drawn, we 9,
		// so all text here goes up by the difference.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		constexpr Point<int16_t> FEET[2] = { Point<int16_t>(227, 101), Point<int16_t>(66, 101) };
		constexpr Point<int16_t> NAME[2] = { Point<int16_t>(229, 107), Point<int16_t>(64, 107) };
		constexpr Point<int16_t> MESO[2] = { Point<int16_t>(281, 243), Point<int16_t>(119, 243) };
		// A side that pressed Trade is shaded half white.
		constexpr Point<int16_t> SHADE[2] = { Point<int16_t>(169, 129), Point<int16_t>(6, 129) };
		constexpr Point<int16_t> SHADE_SIZE = Point<int16_t>(113, 109);

		// The first slot's icon origin and count; the next slots are 39
		// across and 37 down (FUN_007c33cb).
		constexpr int16_t ICON_X[2] = { 171, 9 };
		constexpr int16_t ICON_Y = 161;
		constexpr int16_t COUNT_Y = 149;
		constexpr Point<int16_t> SLOT_PITCH = Point<int16_t>(39, 37);

		// The chat log (FUN_0065f381, FUN_0065fab9): eleven rows of 16 inside
		// its box, wrapped to the box less 14.
		constexpr int16_t LOG_LEFT = 297;
		constexpr int16_t LOG_TOP = 27;
		constexpr int16_t LOG_ROW = 16;
		constexpr int16_t LOG_ROWS = 11;
		constexpr int16_t LOG_WIDTH = 223;
		constexpr size_t LOG_LIMIT = 200;
		constexpr int16_t LOG_CLIP_TOP = 25;
		constexpr int16_t LOG_CLIP_BOTTOM = 217;
		constexpr int16_t SLIDER_X = 542;
		const Range<int16_t> SLIDER_RANGE = Range<int16_t>(23, 215);

		const Rectangle<int16_t> INPUT = Rectangle<int16_t>(296, 488, 245, 260);
		constexpr size_t INPUT_LIMIT = 70;

		constexpr int32_t LOW_LEVEL = 15;
		constexpr int32_t LOW_LEVEL_MESO = 1000000;
		const std::string MESO_LIMIT = "Players that are Level 15 and below \\nmay only trade 1 million mesos per day. "
				"\\nYou have reached the limit today,\\nplease try again tomorrow.";

		// Use, setup and etc items show how many there are.
		bool shows_count(int32_t itemid)
		{
			int32_t prefix = itemid / 1000000;

			return prefix >= 2 && prefix <= 4;
		}
	}

	UITradingRoom::UITradingRoom(const mini_room::Enter &enter) : UIDragElement<PosTRADE>(),
			own_slot(enter.own_slot < 2 ? enter.own_slot : 0), log_rows(0), log_row(0)
	{
		nl::node TradingRoom = NxFiles::ui_classic["UIWindow.img"]["TradingRoom"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = TradingRoom["backgrnd"];

		buttons[Buttons::TRADE] = std::make_unique<MapleButton>(TradingRoom["BtTrade"], BT_TRADE);
		buttons[Buttons::COIN] = std::make_unique<MapleButton>(TradingRoom["BtCoin"], BT_COIN);
		buttons[Buttons::RESET] = std::make_unique<MapleButton>(TradingRoom["BtReset"], BT_RESET);
		buttons[Buttons::ENTER] = std::make_unique<MapleButton>(NxFiles::ui_classic["UIWindow.img"]["Messenger"]["BtEnter"], BT_ENTER);
		buttons[Buttons::CLAIM] = std::make_unique<MapleButton>(Basic["BtClaim"], BT_CLAIM);

		// The names sit on dark tags, in white (FUN_007c27a0).
		for (Member &member : members)
			member.name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		for (const mini_room::Member &member : enter.members)
			visit(member);

		for (Text &label : meso_labels)
			label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK, "0");

		meso_at_open = Stage::get().get_player().get_inventory().get_meso();

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, LOG_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + LOG_ROWS <= log_rows)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, INPUT, INPUT_LIMIT);

		input.set_enter_callback(
				[&](std::string text)
				{
					input.change_text("");
					submit(text);
				}
		);

		input.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					input.set_state(Textfield::State::DISABLED);
					input.set_state(Textfield::State::NORMAL);
				}
		);

		refresh_buttons();

		dimension = backgrnd.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);
	}

	void UITradingRoom::draw(float inter) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		backgrnd.draw(position);

		for (Side side : { Side::MINE, Side::PARTNER })
		{
			const Member &member = members[side == Side::MINE ? own_slot : 1 - own_slot];

			if (member.present)
			{
				// Our own avatar follows our equipment as it changes.
				const CharLook &look = side == Side::MINE ? Stage::get().get_player().get_look() : member.look;

				// The partner on the left faces right, towards me (FUN_007c1f37).
				look.draw(position + FEET[side], side == Side::PARTNER, Stance::Id::STAND1, Expression::Id::DEFAULT);
				member.name.draw(position + NAME[side] + TEXT_Y);
			}

			meso_labels[side].draw(position + MESO[side] + TEXT_Y);

			const mini_room::Offer &offer = offers[side];

			for (int16_t i = 0; i < mini_room::TRADE_SLOTS; i++)
			{
				if (!offer.items[i])
					continue;

				const storage_contents::Stored &item = *offer.items[i];
				int16_t x = ICON_X[side] + SLOT_PITCH.x() * (i % 3);
				int16_t row = SLOT_PITCH.y() * (i / 3);

				ItemData::get(item.itemid).get_icon(false).draw(position + Point<int16_t>(x, ICON_Y + row));

				if (shows_count(item.itemid))
					countset.draw(std::to_string(item.count), position + Point<int16_t>(x, COUNT_Y + row));
			}

			if (offer.confirmed)
				ColorBox(SHADE_SIZE.x(), SHADE_SIZE.y(), Color::Name::WHITE, 0.5f).draw(position + SHADE[side]);
		}

		Range<int16_t> clip = Range<int16_t>(position.y() + LOG_CLIP_TOP, position.y() + LOG_CLIP_BOTTOM);
		int16_t y = LOG_TOP - log_row * LOG_ROW;

		for (const Line &line : log)
		{
			if (y + line.rows * LOG_ROW > LOG_TOP && y < LOG_TOP + LOG_ROWS * LOG_ROW)
				line.text.draw(DrawArgument(position + Point<int16_t>(LOG_LEFT, y) + TEXT_Y), clip);

			y += line.rows * LOG_ROW;
		}

		slider.draw(position);
		input.draw(position + TEXT_Y);

		UIElement::draw_buttons(inter);
	}

	void UITradingRoom::update()
	{
		UIElement::update();

		input.update(position);
	}

	void UITradingRoom::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
		clear_tooltip();
	}

	Cursor::State UITradingRoom::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		if (Cursor::State sstate = slider.send_cursor(cursorpos - position, clicked))
		{
			clear_tooltip();

			return sstate;
		}

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		show_item(cursorpos - position);

		return dstate;
	}

	bool UITradingRoom::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		Point<int16_t> at = cursorpos - position;
		int8_t slot = mini_room::slot_at(at.x(), at.y());

		// Only an empty slot of ours takes an item (FUN_007c359f).
		if (slot > 0 && can_offer() && !offers[Side::MINE].items[slot - 1])
			icon.drop_on_trade(slot);

		return true;
	}

	void UITradingRoom::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UITradingRoom::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			cancel();
	}

	UIElement::Type UITradingRoom::get_type() const
	{
		return TYPE;
	}

	void UITradingRoom::visit(const mini_room::Member &member)
	{
		if (member.slot >= members.size())
			return;

		Member &seat = members[member.slot];
		seat.present = true;
		seat.look = CharLook(member.look);
		seat.name.change_text(member.name);

		refresh_buttons();
	}

	void UITradingRoom::change_look(uint8_t slot, const LookEntry &look)
	{
		if (slot < members.size() && slot != own_slot && members[slot].present)
			members[slot].look = CharLook(look);
	}

	void UITradingRoom::add_chat(uint8_t slot, const std::string &line)
	{
		// My lines are blue, my partner's black.
		if (slot == own_slot)
			add_line(line, Color::Name::SEATBLUE);
		else
			add_line(mini_room::filter_chat(line, CurseData::filter), Color::Name::BLACK);
	}

	void UITradingRoom::put_item(const mini_room::PutItem &put)
	{
		offers[put.side].items[put.slot - 1] = put.item;
	}

	void UITradingRoom::put_meso(const mini_room::PutMeso &put)
	{
		offers[put.side].meso = put.meso;
		meso_labels[put.side].change_text(mini_room::meso_text(put.meso));
	}

	void UITradingRoom::partner_confirmed()
	{
		offers[Side::PARTNER].confirmed = true;

		// v83 answers with the partner's items (FUN_007c20bc).
		TradeConfirmReplyPacket(mini_room::item_ids(offers[Side::PARTNER])).dispatch();
	}

	void UITradingRoom::meso_limit()
	{
		offers[Side::MINE].confirmed = false;
		refresh_buttons();

		UI::get().emplace<UIOk>(MESO_LIMIT, [](bool) {});
	}

	void UITradingRoom::leave(int8_t reason)
	{
		int64_t meso = Stage::get().get_player().get_inventory().get_meso();
		int32_t received = mini_room::received_after_fees(meso_at_open, meso, offers);
		std::string text = mini_room::leave_text(reason, received);

		close();

		if (!text.empty())
			UI::get().emplace<UIOk>(text, [](bool) {});
	}

	Button::State UITradingRoom::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		switch (buttonid)
		{
			case Buttons::TRADE:
				ask_trade();
				break;
			case Buttons::COIN:
				ask_meso();
				break;
			case Buttons::RESET:
				cancel();
				break;
			case Buttons::ENTER:
			{
				std::string text = input.get_text();
				input.change_text("");
				submit(text);
				break;
			}
			default:
				// Like the status bar's, the claim button does nothing.
				break;
		}

		return Button::State::NORMAL;
	}

	void UITradingRoom::cancel()
	{
		MiniRoomLeavePacket().dispatch();

		close();
	}

	void UITradingRoom::close()
	{
		clear_tooltip();

		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
		input.change_text("");

		deactivate();
	}

	void UITradingRoom::request(int32_t cid)
	{
		// Not while transformed, before any other check (FUN_0052fad4).
		if (Stage::get().get_player().shape_refuses())
			return;

		const char *refusal = nullptr;

		// Nor while a dialog is open.
		if (UI::get().dialog_open())
			refusal = "Not available";
		else if (Stage::get().is_player(cid) || !Stage::get().get_character(cid))
			refusal = "Unable to find the character.";

		if (refusal)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(refusal, UIChatBar::LineType::SKILL);

			return;
		}

		TradeCreatePacket().dispatch();
		MiniRoomInvitePacket(cid).dispatch();
	}

	void UITradingRoom::ask_meso()
	{
		Player &player = Stage::get().get_player();

		if (!can_offer() || !player.can_request(ExclRequest::WAIT))
			return;

		auto max = static_cast<int32_t>(std::min<int64_t>(player.get_inventory().get_meso(),
				std::numeric_limits<int32_t>::max()));

		UI::get().emplace<UIEnterNumber>(
				"How much money will you put up?",
				[](int32_t meso)
				{
					Player &player = Stage::get().get_player();

					// v83 checks the daily limit of low levels itself.
					if (player.get_stats().get_stat(MapleStat::Id::LEVEL) <= LOW_LEVEL && meso > LOW_LEVEL_MESO)
					{
						UI::get().emplace<UIOk>(MESO_LIMIT, [](bool) {});

						return;
					}

					player.request(TradePutMesoPacket(meso), ExclRequest::WAIT);
				},
				max, 1
		);
	}

	void UITradingRoom::ask_trade()
	{
		mini_room::TradeQuestions questions = mini_room::trade_questions(offers[Side::PARTNER], item_expiry::now(),
			[](int32_t itemid)
			{
				// v83 cuts names to 100 px (FUN_00988cf3).
				Text name(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(itemid).get_name());
				string_format::format_with_ellipsis(name, 100);

				return name.get_text();
			});

		std::string confirm = questions.confirm;
		auto ask = [confirm]()
		{
			UI::get().emplace<UIYesNo>(
					confirm,
					[](bool yes)
					{
						auto room = UI::get().get_element<UITradingRoom>();

						if (yes && room && room->is_active())
							room->confirm();
					}
			);
		};

		if (questions.time_limit.empty())
			ask();
		else
			UI::get().emplace<UIYesNo>(questions.time_limit, [ask](bool yes)
				{
					if (yes)
						ask();
				});
	}

	void UITradingRoom::confirm()
	{
		if (offers[Side::MINE].confirmed || !members[0].present || !members[1].present)
			return;

		offers[Side::MINE].confirmed = true;
		refresh_buttons();

		TradeConfirmPacket(mini_room::item_ids(offers)).dispatch();
	}

	void UITradingRoom::submit(const std::string &text)
	{
		// The server adds our name and sends the line back.
		if (!text.empty())
			MiniRoomChatPacket(text).dispatch();
	}

	void UITradingRoom::add_line(const std::string &line, Color::Name color)
	{
		bool at_bottom = log_row + LOG_ROWS >= log_rows;

		Text text = Text(Text::Font::A12M, Text::Alignment::LEFT, color, line, LOG_WIDTH, true, 2);
		auto rows = static_cast<int16_t>(std::max(1, (text.height() + LOG_ROW - 3) / LOG_ROW));

		log.push_back({ text, rows });
		log_rows += rows;

		if (log.size() > LOG_LIMIT)
		{
			log_rows -= log.front().rows;
			log_row = std::max<int16_t>(0, log_row - log.front().rows);
			log.erase(log.begin());
		}

		if (at_bottom)
			scroll_to_bottom();
		else
			slider.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UITradingRoom::scroll_to_bottom()
	{
		log_row = std::max<int16_t>(0, log_rows - LOG_ROWS);
		slider.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UITradingRoom::show_item(Point<int16_t> cursor)
	{
		int8_t slot = mini_room::slot_at(cursor.x(), cursor.y());

		if (slot != 0)
		{
			const mini_room::Offer &offer = offers[slot > 0 ? Side::MINE : Side::PARTNER];
			const auto &item = offer.items[std::abs(slot) - 1];

			if (item)
			{
				UI::get().show_item(Tooltip::Parent::TRADINGROOM, item->itemid);

				return;
			}
		}

		clear_tooltip();
	}

	void UITradingRoom::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::TRADINGROOM);
	}

	void UITradingRoom::refresh_buttons()
	{
		bool open = !offers[Side::MINE].confirmed;
		bool full = members[0].present && members[1].present;

		buttons[Buttons::TRADE]->set_state(open && full ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::COIN]->set_state(open ? Button::State::NORMAL : Button::State::DISABLED);
	}

	bool UITradingRoom::can_offer() const
	{
		return !offers[Side::MINE].confirmed && members[0].present && members[1].present;
	}
}
