#include "UIDuey.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Character/Inventory/ItemExpiry.h"
#include "../Data/EquipData.h"
#include "../Data/ItemData.h"

#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The v83 window's layout (CParcelDlg, report b030). Art is copied
		// by its top-left, whatever its origin.
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
	}

	UIDuey::UIDuey(Kind k) : UIDragElement<PosDUEY>(Point<int16_t>(SIZE.x(), 20)), kind(k), offset(0), selection(-1),
							 pending(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Delivery"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		auto top_left = [](nl::node art, Point<int16_t> position)
		{
			return Sprite(art, DrawArgument(position + Texture(art).get_origin()));
		};

		sprites.emplace_back(src["backgrnd"]);
		sprites.push_back(top_left(src["icon0"], TITLE));
		sprites.push_back(top_left(src["npc1"], NPC));

		bubble = src["say1"];
		empty = src["empty"];
		header = src["line0"];
		row_frame = src["line1"];
		footer = src["line2"];
		table = src["receiveTB"];

		auto button = [&](const char *name, Point<int16_t> position)
		{
			nl::node bt = basic[name];

			return std::make_unique<MapleButton>(bt, position + Texture(bt["normal"]["0"]).get_origin());
		};

		buttons[Buttons::BT_CLOSE] = button("BtClose", CLOSE_BUTTON);
		buttons[Buttons::BT_RECEIVE] = button("BtReceive", RECEIVE_BUTTON);
		buttons[Buttons::BT_DELETE] = button("BtDel", DELETE_BUTTON);

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

		dimension = SIZE;
	}

	void UIDuey::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

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

		UIElement::draw_buttons(alpha);
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

			if (row.parcel.item->itemid / 1000000 != 1)
				countset.draw(std::to_string(row.parcel.item->count), position + ITEM_COUNT);
		}

		// A parcel with a message shows it where Duey's words were.
		if (row.parcel.message)
			message.draw(position + MESSAGE - TEXT_RAISE);
		else
			bubble.draw(position + BUBBLE);
	}

	void UIDuey::update()
	{
		UIElement::update();

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

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIDuey::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIDuey::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
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
		pending = false;

		auto iter = std::find_if(rows.begin(), rows.end(), [id](const Row &row) { return row.parcel.id == id; });

		if (iter != rows.end())
		{
			rows.erase(iter);
			offset = std::max<int16_t>(0, std::min<int16_t>(offset, static_cast<int16_t>(rows.size()) - ROWS));
			choose(-1);
			refresh_slider();
		}

		UI::get().emplace<UIOk>(deleted ? "Successfully deleted." : "Successfully claimed.", [](bool) {});
	}

	void UIDuey::added(const parcels::Parcel &parcel)
	{
		if (rows.size() >= parcels::LIMIT)
			return;

		rows.push_back(make_row(parcel));
		refresh_slider();

		UI::get().emplace<UIOk>("A new package has been sent.", [](bool) {});
	}

	void UIDuey::answered(uint8_t mode)
	{
		pending = false;

		if (std::optional<std::string> text = parcels::result_text(mode))
			UI::get().emplace<UIOk>(*text, [](bool) {});
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
			UI::get().emplace<UIOk>("Package is still being sent.\\nPlease try again later.", [](bool) {});

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
					// A map change may have taken the window meanwhile.
					Optional<UIDuey> duey = UI::get().get_element<UIDuey>();

					if (yes && duey && duey->is_active() && !duey->pending)
					{
						duey->pending = true;
						DueyActionPacket(DueyActionPacket::Mode::REMOVE, id).dispatch();
					}
				});
	}

	void UIDuey::close()
	{
		clear_tooltip();
		DueyActionPacket(DueyActionPacket::Mode::CLOSE).dispatch();
		deactivate();
	}
}
