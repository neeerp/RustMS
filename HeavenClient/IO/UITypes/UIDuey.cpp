#include "UIDuey.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Audio/Audio.h"
#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Character/Inventory/ItemExpiry.h"
#include "../Data/CurseData.h"
#include "../Data/EquipData.h"
#include "../Data/ItemData.h"
#include "../Gameplay/Stage.h"

#include "../Net/Handlers/PersonalShop.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <limits>

namespace ms
{
	namespace
	{
		// The v83 window's layout (CParcelDlg, reports b030 and b032). Art
		// is copied by its top-left, whatever its origin.
		// v83 writes text from its top; ours stands 7 lower at the same point
		// (measured on the client).
		const Point<int16_t> TEXT_RAISE = Point<int16_t>(0, 7);

		const Point<int16_t> CLOSE_BUTTON = Point<int16_t>(268, 8);
		const Point<int16_t> RECEIVE_BUTTON = Point<int16_t>(53, 321);
		const Point<int16_t> DELETE_BUTTON = Point<int16_t>(170, 321);
		const Point<int16_t> NPC = Point<int16_t>(18, 110);
		const Point<int16_t> TITLE = Point<int16_t>(13, 64);
		const Point<int16_t> BUBBLE = Point<int16_t>(95, 114);
		const Point<int16_t> EMPTY = Point<int16_t>(66, 240);
		const Point<int16_t> MESSAGE = Point<int16_t>(95, 115);
		constexpr int16_t MESSAGE_WIDTH = 165;

		// The tab strip (CCtrlTab, style 1): each tab as wide as its label
		// less 4, a 4 px cap at each end and an 8 px joint between.
		const Point<int16_t> TAB_STRIP = Point<int16_t>(6, 23);
		constexpr int16_t TAB_HEIGHT = 19;
		constexpr int16_t TAB_CAP = 4;
		constexpr int16_t TAB_JOINT = 8;
		// Clicks count from 4 below the strip's top, and 2 px past a tab's
		// sides (FUN_004dd827).
		constexpr int16_t TAB_CLICK_TOP = 4;
		constexpr int16_t TAB_CLICK_SIDE = 2;

		// The list: a header, three rows 18 high, and a closing line.
		const Point<int16_t> HEADER = Point<int16_t>(8, 210);
		const Point<int16_t> FIRST_ROW = Point<int16_t>(8, 229);
		const Point<int16_t> FOOTER = Point<int16_t>(8, 283);
		constexpr int16_t ROWS = 3;
		constexpr int16_t ROW_HEIGHT = 18;
		const Point<int16_t> SELECTION = Point<int16_t>(1, 0);
		constexpr int16_t SELECTION_WIDTH = 254;
		// Within a row: the name and status, 3 below its top.
		const Point<int16_t> NAME = Point<int16_t>(5, 3);
		const Point<int16_t> STATUS = Point<int16_t>(180, 3);
		// Clicks choose a row from here (FUN_006f0b72).
		const Rectangle<int16_t> ROW_CLICKS = Rectangle<int16_t>(10, 256, 233, 287);
		constexpr int16_t ROW_CLICK_TOP = 230;

		constexpr int16_t SLIDER_X = 267;
		// 85 high, the next arrow included.
		const Range<int16_t> SLIDER = Range<int16_t>(210, 210 + 85 - 13);

		// The chosen parcel, under the list.
		const Point<int16_t> TABLE = Point<int16_t>(0, 349);
		const Point<int16_t> SENDER = Point<int16_t>(126, 359);
		const Point<int16_t> MESO = Point<int16_t>(126, 378);
		const Point<int16_t> ITEM_NAME = Point<int16_t>(126, 409);
		const Point<int16_t> LEVEL = Point<int16_t>(126, 428);
		const Point<int16_t> ITEM_ICON = Point<int16_t>(43, 440);
		const Point<int16_t> ITEM_COUNT = Point<int16_t>(43, 428);
		// v83 tests for the item's tooltip 16 above its icon (FUN_006f0bd6).
		const Rectangle<int16_t> ITEM_HOVER = Rectangle<int16_t>(42, 76, 392, 426);

		// The send panels (FUN_006f301b, FUN_006f15ab).
		const Point<int16_t> SEND_BUTTON = Point<int16_t>(200, 321);
		const Point<int16_t> COIN_BUTTON = Point<int16_t>(121, 232);
		const Point<int16_t> TABLE_BUTTON = Point<int16_t>(12, 323);
		const Point<int16_t> SEND_NPC = Point<int16_t>(29, 110);
		const Point<int16_t> SEND_LABEL = Point<int16_t>(36, 319);
		const Point<int16_t> SEND_BOX = Point<int16_t>(8, 211);
		const Point<int16_t> OUT_MESO = Point<int16_t>(141, 234);
		const Point<int16_t> OUT_ICON = Point<int16_t>(122, 285);
		const Point<int16_t> OUT_COUNT = Point<int16_t>(122, 273);
		const Point<int16_t> OUT_NAME = Point<int16_t>(164, 254);
		const Point<int16_t> OUT_LEVEL = Point<int16_t>(165, 273);
		const Rectangle<int16_t> OUT_SLOT = Rectangle<int16_t>(121, 155, 252, 286);
		const Rectangle<int16_t> MESO_FIELD = Rectangle<int16_t>(137, 281, 232, 247);
		const Rectangle<int16_t> MESSAGE_AREA = Rectangle<int16_t>(95, 264, 115, 183);
		// The quick panel cuts the item's name to this (FUN_00988cf3).
		constexpr size_t QUICK_NAME_WIDTH = 104;
		const Point<int16_t> HINTS[2] = { Point<int16_t>(89, 122), Point<int16_t>(89, 135) };
		// With its fee table folded away, a send panel ends here.
		constexpr int16_t FOLDED_HEIGHT = 350;

		// v83's edits are at (123, 213), 154 by 14, and (95, 115), 165 by 68;
		// our text sits 3 px lower than v83's.
		const Rectangle<int16_t> RECIPIENT_BOX = Rectangle<int16_t>(123, 277, 210, 224);
		const Rectangle<int16_t> MESSAGE_BOX = Rectangle<int16_t>(95, 260, 112, 180);
		// I: the recipient's limit is a character name's.
		constexpr size_t RECIPIENT_LIMIT = 12;
		constexpr size_t MESSAGE_LIMIT = 100;
		constexpr size_t MESSAGE_ROWS = 5;
		constexpr int16_t MESSAGE_LINE = 13;

		constexpr int32_t QUICK_COUPON = 5330000;

		const Color::Name MESO_COLORS[] = {
			Color::Name::PRICEBLUE, Color::Name::PRICETEAL, Color::Name::PRICEGREEN,
			Color::Name::PRICEORANGE, Color::Name::PRICEPINK, Color::Name::PRICEPLUM
		};

		const Point<int16_t> SIZE = Point<int16_t>(290, 541);

		void scroll(int16_t &offset, int16_t size, bool upwards)
		{
			int16_t next = offset + (upwards ? -1 : 1);

			if (next >= 0 && next <= size - ROWS)
				offset = next;
		}

		Text label(Color::Name color, const std::string &text = "")
		{
			return Text(Text::Font::A12M, Text::Alignment::LEFT, color, text);
		}

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		// The window, if a map change hasn't taken it meanwhile.
		Optional<UIDuey> open_window()
		{
			Optional<UIDuey> duey = UI::get().get_element<UIDuey>();

			return duey && duey->is_active() ? duey : Optional<UIDuey>();
		}
	}

	UIDuey::UIDuey(Kind k) : UIDragElement<PosDUEY>(Point<int16_t>(SIZE.x(), 20)), kind(k), offset(0), selection(-1),
							 pending(false), touched(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Delivery"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		sprites.emplace_back(src["backgrnd"]);

		receive_title = src["icon0"];
		receive_npc = src["npc1"];
		bubble = src["say1"];
		empty = src["empty"];
		header = src["line0"];
		row_frame = src["line1"];
		footer = src["line2"];
		table = src["receiveTB"];

		send_title = src["icon1"];
		send_npc = src["npc0"];
		send_bubble = src["say0"];
		send_label = src["icon2"];
		send_box = src["sendTB"];
		quick_title = src["icon3"];
		quick_npc = src["npc2"];
		fees[0] = src["tax"];
		fees[1] = src["tax1"];

		nl::node strip = basic["Tab2"];

		for (size_t i = 0; i < 2; i++)
		{
			tab_left[i] = strip["left" + std::to_string(i)];
			tab_right[i] = strip["right" + std::to_string(i)];
			tab_fill[i] = strip["fill" + std::to_string(i)];
		}

		for (size_t i = 0; i < 3; i++)
		{
			tab_middle[i] = strip["middle" + std::to_string(i)];
			tab_label[0][i] = src["Tab"]["disabled"][std::to_string(i)];
			tab_label[1][i] = src["Tab"]["enabled"][std::to_string(i)];
		}

		auto button = [&](const char *name, Point<int16_t> position)
		{
			nl::node bt = basic[name];

			return std::make_unique<MapleButton>(bt, position + Texture(bt["normal"]["0"]).get_origin());
		};

		buttons[Buttons::BT_CLOSE] = button("BtClose", CLOSE_BUTTON);
		buttons[Buttons::BT_RECEIVE] = button("BtReceive", RECEIVE_BUTTON);
		buttons[Buttons::BT_DELETE] = button("BtDel", DELETE_BUTTON);
		buttons[Buttons::BT_SEND] = button("BtSend", SEND_BUTTON);
		buttons[Buttons::BT_COIN] = button("BtCoin", COIN_BUTTON);
		buttons[Buttons::BT_MAX] = button("BtMax2", TABLE_BUTTON);
		buttons[Buttons::BT_MIN] = button("BtMin2", TABLE_BUTTON);

		selection_box = ColorBox(SELECTION_WIDTH, ROW_HEIGHT, Color::Name::PARCELSELECT, 1.0f);

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, ROWS,
				[&](bool upwards)
				{
					scroll(offset, static_cast<int16_t>(rows.size()), upwards);
				}
		);

		sender = label(Color::Name::BLACK);
		meso = label(Color::Name::BLACK);
		item_name = label(Color::Name::BLACK);
		level = label(Color::Name::BLACK);
		message = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "", MESSAGE_WIDTH);

		recipient = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, RECIPIENT_BOX,
			RECIPIENT_LIMIT);
		quick_message = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, MESSAGE_BOX,
			MESSAGE_LIMIT);
		quick_message.set_lines(MESSAGE_BOX.width(), MESSAGE_ROWS, MESSAGE_LINE);

		// Enter sends from the normal panel only (FUN_006f50fc).
		recipient.set_enter_callback(
				[&](std::string)
				{
					if (tab == Tab::SEND)
						send();
				});

		for (Textfield *field : { &recipient, &quick_message })
			field->set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });

		hint[0] = label(Color::Name::DARKGREY, "Please enter the message to send");
		hint[1] = label(Color::Name::DARKGREY, "by clicking here (100 letters)");

		// Duey's own window has all three tabs; the others one each
		// (FUN_006f4a50).
		switch (kind)
		{
			case Kind::PARCELS:
				tabs = { Tab::RECEIVE, Tab::SEND, Tab::QUICK };
				break;
			case Kind::QUICK_SEND:
				tabs = { Tab::QUICK };
				break;
			case Kind::RECEIVE:
				tabs = { Tab::RECEIVE };
				break;
		}

		tab = tabs.front();
		show_controls();

		dimension = SIZE;
	}

	void UIDuey::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		draw_tabs();

		if (tab == Tab::RECEIVE)
		{
			copy(receive_title, TITLE);
			copy(receive_npc, NPC);

			if (rows.empty())
			{
				bubble.draw(position + BUBBLE);
				empty.draw(position + EMPTY);
			} else
			{
				draw_list();

				if (const Row *row = chosen())
				{
					table.draw(position + TABLE);
					draw_chosen(*row);
				} else
				{
					bubble.draw(position + BUBBLE);
				}
			}

			slider.draw(position);
		} else
		{
			if (tab == Tab::SEND)
			{
				copy(send_bubble, BUBBLE);
				copy(send_npc, SEND_NPC);
				copy(send_title, TITLE);
				copy(send_label, SEND_LABEL);
			} else
			{
				copy(quick_title, TITLE);
				copy(quick_npc, NPC);
			}

			copy(send_box, SEND_BOX);
			draw_outgoing(outgoing(tab));

			if (tab == Tab::QUICK && !touched)
				for (size_t i = 0; i < 2; i++)
					hint[i].draw(position + HINTS[i] - TEXT_RAISE);

			recipient.draw(position);
			quick_message.draw(position);
		}

		UIElement::draw_buttons(alpha);
	}

	void UIDuey::copy(const Texture &art, Point<int16_t> at) const
	{
		art.draw(position + at + art.get_origin());
	}

	void UIDuey::draw_tabs() const
	{
		Point<int16_t> strip = position + TAB_STRIP;
		std::vector<TabBox> boxes = tab_boxes();

		tab_left[boxes.front().tab == tab].draw(strip);

		for (size_t k = 0; k < boxes.size(); k++)
		{
			const TabBox &box = boxes[k];
			bool current = box.tab == tab;

			if (k + 1 < boxes.size())
			{
				size_t variant = current ? 1 : boxes[k + 1].tab == tab ? 2 : 0;
				tab_middle[variant].draw(strip + Point<int16_t>(box.end, 0));
			} else
			{
				tab_right[current].draw(strip + Point<int16_t>(box.end, 0));
			}

			for (int16_t x = box.start; x < box.end; x++)
				tab_fill[current].draw(strip + Point<int16_t>(x, 0));

			const Texture &name = tab_label[current][box.tab];
			int16_t y = 2 + TAB_HEIGHT / 2 - (name.get_dimensions().y() >> 1);
			name.draw(strip + Point<int16_t>(box.start - 2, y));
		}
	}

	std::vector<UIDuey::TabBox> UIDuey::tab_boxes() const
	{
		std::vector<TabBox> boxes;
		int16_t cursor = TAB_CAP;

		for (Tab each : tabs)
		{
			int16_t width = tab_label[each == tab][each].get_dimensions().x() - 4;
			boxes.push_back({ each, cursor, static_cast<int16_t>(cursor + width) });
			cursor += width + TAB_JOINT;
		}

		return boxes;
	}

	void UIDuey::draw_list() const
	{
		header.draw(position + HEADER);

		for (int16_t i = 0; i < ROWS; i++)
		{
			int16_t index = offset + i;
			Point<int16_t> row = position + FIRST_ROW + Point<int16_t>(0, ROW_HEIGHT * i);
			bool chosen = index == selection;

			if (chosen)
				selection_box.draw(DrawArgument(row + SELECTION));

			row_frame.draw(row);

			if (index >= static_cast<int16_t>(rows.size()))
				continue;

			const Row &entry = rows[index];
			entry.name[chosen].draw(row + NAME - TEXT_RAISE);
			entry.status[chosen].draw(row + STATUS - TEXT_RAISE);
		}

		footer.draw(position + FOOTER);
	}

	void UIDuey::draw_chosen(const Row &row) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		sender.draw(position + SENDER - TEXT_RAISE);

		if (row.parcel.meso != 0)
			meso.draw(position + MESO - TEXT_RAISE);

		if (row.parcel.item)
		{
			item_icon.draw(position + ITEM_ICON);
			item_name.draw(position + ITEM_NAME - TEXT_RAISE);
			level.draw(position + LEVEL - TEXT_RAISE);

			if (!parcels::single(row.parcel.item->itemid))
				countset.draw(std::to_string(row.parcel.item->count), position + ITEM_COUNT);
		}

		// A parcel with a message shows it where Duey's words were.
		if (row.parcel.message)
			message.draw(position + MESSAGE - TEXT_RAISE);
		else
			bubble.draw(position + BUBBLE);
	}

	void UIDuey::draw_outgoing(const Outgoing &out) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		if (out.meso != 0)
			out.meso_text.draw(position + OUT_MESO - TEXT_RAISE);

		if (out.item_id != 0)
		{
			out.icon.draw(position + OUT_ICON);

			if (!parcels::single(out.item_id))
				countset.draw(std::to_string(out.count), position + OUT_COUNT);

			out.item_name.draw(position + OUT_NAME - TEXT_RAISE);
			out.level.draw(position + OUT_LEVEL - TEXT_RAISE);
		}

		if (out.table)
			copy(fees[&out == &outgoings[1]], TABLE);
	}

	void UIDuey::update()
	{
		UIElement::update();

		recipient.update(position);
		quick_message.update(position);

		// A notice replaces the one before, so each waits for the last to close.
		Optional<UIOk> notice = UI::get().get_element<UIOk>();

		if (!expired.empty() && !(notice && notice->is_active()))
		{
			UI::get().emplace<UIOk>(expired.front(), [](bool) {});
			expired.pop_front();
		}

		int64_t now = item_expiry::now();

		for (Row &row : rows)
			row.set_status(now);
	}

	Button::State UIDuey::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_CLOSE:
				close();
				break;
			case Buttons::BT_RECEIVE:
				claim();
				break;
			case Buttons::BT_DELETE:
				remove_chosen();
				break;
			case Buttons::BT_SEND:
				if (tab == Tab::SEND)
					send();
				else
					send_quick();

				break;
			case Buttons::BT_COIN:
				ask_mesos();
				break;
			case Buttons::BT_MAX:
			case Buttons::BT_MIN:
				toggle_table();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIDuey::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
		clear_tooltip();
	}

	Cursor::State UIDuey::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursoroffset = cursorpos - position;

		if (tab == Tab::RECEIVE)
		{
			if (slider.isenabled())
			{
				Cursor::State state = slider.send_cursor(cursoroffset, clicked);

				if (state != Cursor::State::IDLE)
				{
					clear_tooltip();

					return state;
				}
			}

			show_item(cursoroffset);

			if (clicked && !dragged && ROW_CLICKS.contains(cursoroffset))
				choose(static_cast<int16_t>(offset + (cursoroffset.y() - ROW_CLICK_TOP) / ROW_HEIGHT));
		} else
		{
			Cursor::State state = send_panel_cursor(clicked, cursorpos);

			if (state != Cursor::State::IDLE)
				return state;
		}

		if (clicked && !dragged && click_tab(cursoroffset))
			return Cursor::State::IDLE;

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	Cursor::State UIDuey::send_panel_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursoroffset = cursorpos - position;
		Cursor::State state = Cursor::State::IDLE;

		// Each edit hears every click, which takes the caret from the other.
		for (Textfield *field : { &recipient, &quick_message })
		{
			Cursor::State over = field->send_cursor(cursorpos, clicked);

			if (over == Cursor::State::IDLE)
				continue;

			// Clicking the message hides its hint for good (FUN_006f4ed5).
			if (field == &quick_message && clicked)
				touched = true;

			state = over;
		}

		if (state != Cursor::State::IDLE || dragged)
			return state;

		Outgoing &out = outgoing(tab);
		bool on_slot = OUT_SLOT.contains(cursoroffset);

		if (on_slot && out.item_id != 0)
		{
			if (out.tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::DUEY, out.slot);
			else
				UI::get().show_item(Tooltip::Parent::DUEY, out.item_id);
		} else
		{
			clear_tooltip();
		}

		bool on_meso = MESO_FIELD.contains(cursoroffset);

		if (clicked && !pending)
		{
			if (on_slot && out.item_id != 0)
			{
				Tab panel = tab;

				UI::get().emplace<UIYesNo>(
						"Are you sure you want to cancel?",
						[panel](bool yes)
						{
							if (Optional<UIDuey> duey = open_window(); yes && duey)
								duey->take_back(panel);
						});

				return Cursor::State::IDLE;
			}

			if (on_meso)
			{
				ask_mesos();

				return Cursor::State::IDLE;
			}
		}

		bool on_message = tab == Tab::QUICK && MESSAGE_AREA.contains(cursoroffset);

		if (on_slot || on_meso || on_message)
			return Cursor::State::CANCLICK;

		return Cursor::State::IDLE;
	}

	bool UIDuey::click_tab(Point<int16_t> cursor)
	{
		Point<int16_t> at = cursor - TAB_STRIP;

		if (pending || at.y() < TAB_CLICK_TOP || at.y() >= TAB_HEIGHT)
			return false;

		for (const TabBox &box : tab_boxes())
		{
			if (at.x() >= box.start - TAB_CLICK_SIDE && at.x() <= box.end + TAB_CLICK_SIDE)
			{
				Sound(Sound::Name::TAB).play();
				select_tab(box.tab);

				return true;
			}
		}

		return false;
	}

	void UIDuey::select_tab(Tab next)
	{
		tab = next;

		// Only the quick message goes, even on the tab already open
		// (FUN_006f5260).
		if (next != Tab::SEND)
			quick_message.change_text("");

		clear_tooltip();
		show_controls();
	}

	void UIDuey::show_controls()
	{
		bool receiving = tab == Tab::RECEIVE;
		bool folded = !receiving && !outgoing(tab).table;

		buttons[Buttons::BT_RECEIVE]->set_active(receiving);
		buttons[Buttons::BT_DELETE]->set_active(receiving);
		buttons[Buttons::BT_SEND]->set_active(!receiving);
		buttons[Buttons::BT_COIN]->set_active(!receiving);
		// The open table shows BtMax2 (id 0x7e6 at +0xbc), the folded one
		// BtMin2 (FUN_006f52be).
		buttons[Buttons::BT_MAX]->set_active(!receiving && !folded);
		buttons[Buttons::BT_MIN]->set_active(!receiving && folded);

		Textfield::State state = pending ? Textfield::State::DISABLED : Textfield::State::NORMAL;
		recipient.set_state(receiving ? Textfield::State::DISABLED : state);
		quick_message.set_state(tab == Tab::QUICK ? state : Textfield::State::DISABLED);
	}

	bool UIDuey::send_icon(const Icon &icon, Point<int16_t>)
	{
		icon.drop_on_duey();

		return true;
	}

	bool UIDuey::is_in_range(Point<int16_t> cursorpos) const
	{
		// A send panel with its table folded lets clicks through below it.
		if (tab != Tab::RECEIVE && !outgoing(tab).table && cursorpos.y() - position.y() > FOLDED_HEIGHT)
			return false;

		return UIElement::is_in_range(cursorpos);
	}

	void UIDuey::send_scroll(double yoffset)
	{
		if (tab == Tab::RECEIVE)
			slider.send_scroll(yoffset);
	}

	void UIDuey::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN && tab == Tab::RECEIVE)
			claim();
		else if (keycode == KeyAction::Id::RETURN && tab == Tab::SEND)
			send();
	}

	UIElement::Type UIDuey::get_type() const
	{
		return TYPE;
	}

	void UIDuey::open(const parcels::Lists &lists)
	{
		rows.clear();

		for (const parcels::Parcel &parcel : lists.waiting)
			if (rows.size() < parcels::LIMIT)
				rows.push_back(make_row(parcel));

		offset = 0;
		choose(-1);
		refresh_slider();

		// With nothing waiting, Duey's own window opens on sending
		// (FUN_006f56ea).
		if (kind == Kind::PARCELS && rows.empty())
			select_tab(Tab::SEND);

		// One notice for each parcel that expired (FUN_006ef69c).
		for (const parcels::Parcel &parcel : lists.expired)
		{
			std::string text = "The package sent by [" + parcel.sender + "]\\nhas been expired.\\n";

			if (parcel.meso != 0)
				text += "\\nmeso : " + std::to_string(parcel.meso);

			if (parcel.item)
				text += "\\nitem : " + ItemData::get(parcel.item->itemid).get_name();

			expired.push_back(text);
		}
	}

	void UIDuey::removed(int32_t id, bool deleted)
	{
		wait(false);

		auto iter = std::find_if(rows.begin(), rows.end(), [id](const Row &row) { return row.parcel.id == id; });

		if (iter != rows.end())
		{
			rows.erase(iter);
			offset = std::max<int16_t>(0, std::min<int16_t>(offset, static_cast<int16_t>(rows.size()) - ROWS));
			choose(-1);
			refresh_slider();
		}

		notice(deleted ? "Successfully deleted." : "Successfully claimed.");
	}

	void UIDuey::added(const parcels::Parcel &parcel)
	{
		if (rows.size() >= parcels::LIMIT)
			return;

		rows.push_back(make_row(parcel));
		refresh_slider();

		notice("A new package has been sent.");
	}

	void UIDuey::answered(uint8_t mode)
	{
		wait(false);

		std::optional<std::string> text = parcels::result_text(mode);

		if (text)
			notice(*text);

		// A parcel sent empties both panels; the coupon's window is done
		// (FUN_006f5b7e).
		if (mode == 0x12)
		{
			clear_outgoing();

			if (kind == Kind::QUICK_SEND)
				close();
		}
	}

	void UIDuey::offer(InventoryType::Id from, int16_t slot, int32_t item_id, int16_t count)
	{
		// The parcels' tab takes nothing (FUN_006f520e).
		if (tab == Tab::RECEIVE || pending)
			return;

		Tab panel = tab;

		// A stack asks how many; cancelling puts nothing in (FUN_006f3cf6).
		if (parcels::single(item_id))
		{
			put(panel, from, slot, item_id, 1);

			return;
		}

		UI::get().emplace<UIEnterNumber>(
				"How many do you wish to send?",
				[panel, from, slot, item_id](int32_t number)
				{
					Optional<UIDuey> duey = open_window();

					if (duey && !duey->pending && duey->tab == panel)
						duey->put(panel, from, slot, item_id, static_cast<int16_t>(number));
				},
				count, 1);
	}

	UIDuey::Row UIDuey::make_row(const parcels::Parcel &parcel)
	{
		Row row;
		row.parcel = parcel;
		row.name[0] = label(Color::Name::BLACK, parcel.sender);
		row.name[1] = label(Color::Name::WHITE, parcel.sender);
		row.set_status(item_expiry::now());

		return row;
	}

	void UIDuey::Row::set_status(int64_t now)
	{
		std::string text = parcels::status(parcel.expires, now);

		if (text == shown)
			return;

		shown = text;

		// One about to expire is red, even when chosen.
		bool expiring = parcels::days_left(parcel.expires, now) < 1;
		status[0] = label(expiring ? Color::Name::PARCELRED : Color::Name::BLACK, text);
		status[1] = label(expiring ? Color::Name::PARCELRED : Color::Name::WHITE, text);
	}

	const UIDuey::Row *UIDuey::chosen() const
	{
		if (selection < 0 || selection >= static_cast<int16_t>(rows.size()))
			return nullptr;

		return &rows[selection];
	}

	void UIDuey::choose(int16_t index)
	{
		selection = index < static_cast<int16_t>(rows.size()) ? index : -1;

		const Row *row = chosen();

		if (!row)
			return;

		const parcels::Parcel &parcel = row->parcel;
		sender.change_text(parcel.sender);

		std::string mesos = std::to_string(parcel.meso);
		string_format::split_number(mesos);
		meso.change_text(mesos);

		// An empty message says so.
		std::string words = parcel.message.value_or("");
		message.change_text(words.empty() ? "No information entered." : words);

		if (parcel.item)
		{
			int32_t itemid = parcel.item->itemid;
			const ItemData &data = ItemData::get(itemid);
			item_icon = data.get_icon(false);
			item_name.change_text(data.get_name());

			int16_t required = itemid / 1000000 == 1 ? EquipData::get(itemid).get_reqstat(MapleStat::Id::LEVEL) : 0;

			if (required > 0)
				level = label(Color::Name::PARCELRED, "Over Lv " + std::to_string(required) + " only");
			else
				level = label(Color::Name::BLACK, "No level limit");
		}
	}

	void UIDuey::refresh_slider()
	{
		auto size = static_cast<int16_t>(rows.size());
		slider.setrows(offset, ROWS, std::max<int16_t>(size, ROWS));
	}

	void UIDuey::show_item(Point<int16_t> cursor)
	{
		const Row *row = chosen();

		if (row && row->parcel.item && ITEM_HOVER.contains(cursor))
			UI::get().show_item(Tooltip::Parent::DUEY, row->parcel.item->itemid);
		else
			clear_tooltip();
	}

	void UIDuey::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::DUEY);
	}

	void UIDuey::claim()
	{
		const Row *row = chosen();

		if (pending || !row)
			return;

		if (parcels::delivering(row->parcel.expires, item_expiry::now()))
		{
			notice("Package is still being sent.\\nPlease try again later.");

			return;
		}

		pending = true;
		DueyActionPacket(DueyActionPacket::Mode::CLAIM, row->parcel.id).dispatch();
	}

	void UIDuey::remove_chosen()
	{
		const Row *row = chosen();

		if (pending || !row)
			return;

		int32_t id = row->parcel.id;

		UI::get().emplace<UIYesNo>(
				"The items and/or mesos included in the package\\nwill be permanently deleted.\\nAre you sure you want "
				"to delete it?",
				[id](bool yes)
				{
					Optional<UIDuey> duey = open_window();

					if (yes && duey && !duey->pending)
					{
						duey->pending = true;
						DueyActionPacket(DueyActionPacket::Mode::REMOVE, id).dispatch();
					}
				});
	}

	UIDuey::Outgoing &UIDuey::outgoing(Tab panel)
	{
		return outgoings[panel == Tab::QUICK];
	}

	const UIDuey::Outgoing &UIDuey::outgoing(Tab panel) const
	{
		return outgoings[panel == Tab::QUICK];
	}

	void UIDuey::ask_mesos()
	{
		if (pending || tab == Tab::RECEIVE)
			return;

		int64_t purse = std::min<int64_t>(Stage::get().get_player().get_inventory().get_meso(),
			std::numeric_limits<int32_t>::max());

		// A normal send keeps its fee back; a quick one has none
		// (FUN_006f3875, FUN_006f20a5).
		if (tab == Tab::SEND && purse <= parcels::SEND_FEE)
		{
			notice("You do not have enough mesos.");

			return;
		}

		int32_t left = static_cast<int32_t>(tab == Tab::SEND ? purse - parcels::SEND_FEE : purse);
		Tab panel = tab;

		UI::get().emplace<UIEnterNumber>(
				"How much money will you put up?",
				[panel](int32_t number)
				{
					if (Optional<UIDuey> duey = open_window())
						duey->set_mesos(panel, number);
				},
				parcels::most_mesos(left), 0, 0);
	}

	void UIDuey::set_mesos(Tab panel, int32_t amount)
	{
		const CharStats &stats = Stage::get().get_player().get_stats();

		if (stats.get_stat(MapleStat::Id::LEVEL) <= parcels::LOW_LEVEL && amount > parcels::LOW_LEVEL_MESOS)
		{
			notice(*parcels::result_text(0x11));
			amount = 0;
		}

		Outgoing &out = outgoing(panel);
		out.meso = amount;

		std::string text = std::to_string(amount);
		string_format::split_number(text);
		out.meso_text = label(MESO_COLORS[personal_shop::price_tier(amount)], text + " mesos.");
	}

	void UIDuey::put(Tab panel, InventoryType::Id from, int16_t slot, int32_t item_id, int16_t count)
	{
		Outgoing &out = outgoing(panel);
		out.tab = from;
		out.slot = slot;
		out.item_id = item_id;
		out.count = count;

		const ItemData &data = ItemData::get(item_id);
		out.icon = data.get_icon(false);
		out.item_name = label(Color::Name::BLACK, data.get_name());

		if (panel == Tab::QUICK)
			string_format::format_with_ellipsis(out.item_name, QUICK_NAME_WIDTH);

		int16_t required = item_id / 1000000 == 1 ? EquipData::get(item_id).get_reqstat(MapleStat::Id::LEVEL) : 0;

		if (required > 0)
			out.level = label(Color::Name::PARCELRED, "Over Lv " + std::to_string(required) + " only");
		else
			out.level = label(Color::Name::BLACK, "No level limit");
	}

	void UIDuey::take_back(Tab panel)
	{
		Outgoing &out = outgoing(panel);
		out.tab = InventoryType::Id::NONE;
		out.slot = 0;
		out.item_id = 0;
		out.count = 0;
		clear_tooltip();
	}

	bool UIDuey::still_offered(Tab panel)
	{
		const Outgoing &out = outgoing(panel);

		if (out.item_id == 0)
			return true;

		// HC keeps the slot where v83 keeps the item, so a move since it was
		// offered (a sort, a swap, a potion used) takes it back.
		const Inventory &inventory = Stage::get().get_player().get_inventory();

		if (inventory.get_item_id(out.tab, out.slot) == out.item_id
			&& inventory.get_item_count(out.tab, out.slot) >= out.count)
			return true;

		take_back(panel);
		notice(*parcels::result_text(0x0b));

		return false;
	}

	void UIDuey::toggle_table()
	{
		if (tab == Tab::RECEIVE)
			return;

		Outgoing &out = outgoing(tab);
		out.table = !out.table;
		show_controls();
	}

	void UIDuey::send()
	{
		const Outgoing &out = outgoing(Tab::SEND);

		if (pending)
			return;

		if (out.item_id == 0 && out.meso == 0)
		{
			notice("You must select either Mesos or items to send.");

			return;
		}

		std::string to = recipient.get_text();

		if (to.empty())
		{
			notice("Please enter the name of the recipient.");

			return;
		}

		int32_t fee = parcels::SEND_FEE + parcels::tax(out.meso);

		UI::get().emplace<UIYesNo>(
				"The total wiring/transportation fee is\\n" + std::to_string(fee) +
					" mesos.\\nAre you sure you want to send the package?",
				[to](bool yes)
				{
					Optional<UIDuey> duey = open_window();

					if (!yes || !duey || duey->pending)
						return;

					if (!duey->still_offered(Tab::SEND))
						return;

					const Outgoing &sent = duey->outgoing(Tab::SEND);
					DueyActionPacket(sent.tab, sent.slot, sent.count, sent.meso, to).dispatch();
					duey->wait(true);
				});
	}

	void UIDuey::send_quick()
	{
		const Outgoing &out = outgoing(Tab::QUICK);

		if (pending)
			return;

		int16_t coupon = Stage::get().get_player().get_inventory().find_item(InventoryType::Id::CASH, QUICK_COUPON);

		if (coupon <= 0)
		{
			notice("You do not have the Quick Delivery Coupon.");

			return;
		}

		if (out.item_id == 0 && out.meso == 0)
		{
			notice("You must select either Mesos or items to send.");

			return;
		}

		std::string to = recipient.get_text();

		if (to.empty())
		{
			notice("Please enter the name of the recipient.");

			return;
		}

		std::string words = quick_message.get_text();

		if (!CurseData::valid(words))
		{
			notice("Cursing is not allowed.");

			return;
		}

		UI::get().emplace<UIYesNo>(
				"The service charge \\nwill cost " + std::to_string(parcels::tax(out.meso)) +
					" mesos.\\nThis will also use up \\n1 [Quick Delivery Coupon].\\nAre you sure \\nyou want to send it?",
				[to, words, coupon](bool yes)
				{
					Optional<UIDuey> duey = open_window();

					if (!yes || !duey || duey->pending)
						return;

					if (!duey->still_offered(Tab::QUICK))
						return;

					const Outgoing &sent = duey->outgoing(Tab::QUICK);
					DueyActionPacket(sent.tab, sent.slot, sent.count, sent.meso, to, CurseData::filter(words), coupon)
						.dispatch();
					duey->wait(true);
				});
	}

	void UIDuey::clear_outgoing()
	{
		for (Tab panel : { Tab::SEND, Tab::QUICK })
		{
			take_back(panel);
			outgoing(panel).meso = 0;
		}

		recipient.change_text("");
		quick_message.change_text("");
		touched = false;
	}

	void UIDuey::wait(bool waiting)
	{
		pending = waiting;

		// The controls rest until the answer (FUN_006f5557), and the quick
		// message goes.
		for (uint16_t id : { Buttons::BT_RECEIVE, Buttons::BT_DELETE, Buttons::BT_SEND, Buttons::BT_COIN,
							 Buttons::BT_MAX, Buttons::BT_MIN })
			buttons[id]->set_state(waiting ? Button::State::DISABLED : Button::State::NORMAL);

		if (waiting)
			quick_message.change_text("");

		show_controls();
	}

	void UIDuey::close()
	{
		clear_tooltip();
		recipient.set_state(Textfield::State::DISABLED);
		quick_message.set_state(Textfield::State::DISABLED);
		DueyActionPacket(DueyActionPacket::Mode::CLOSE).dispatch();
		deactivate();
	}
}
