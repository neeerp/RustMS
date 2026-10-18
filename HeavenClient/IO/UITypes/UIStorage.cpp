#include "UIStorage.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Audio/Audio.h"
#include "../Data/ItemData.h"

#include "../Net/Handlers/ShopList.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The v83 dialog's layout (CTrunkDlg::OnCreate and Draw).
		constexpr Point<int16_t> EXIT_BUTTON = Point<int16_t>(154, 15);
		constexpr Point<int16_t> GET_BUTTON = Point<int16_t>(154, 35);
		constexpr Point<int16_t> SORT_BUTTON = Point<int16_t>(154, 55);
		constexpr Point<int16_t> PUT_BUTTON = Point<int16_t>(384, 15);
		constexpr Point<int16_t> OUT_COIN_BUTTON = Point<int16_t>(5, 294);
		constexpr Point<int16_t> IN_COIN_BUTTON = Point<int16_t>(236, 294);
		constexpr Point<int16_t> NPC_FEET = Point<int16_t>(53, 76);
		// The right ends of the stored and carried mesos.
		constexpr Point<int16_t> STORED_MESO = Point<int16_t>(190, 296);
		constexpr Point<int16_t> CARRIED_MESO = Point<int16_t>(418, 296);
		constexpr Point<int16_t> TABS = Point<int16_t>(234, 92);
		constexpr int16_t TAB_FILL = 32;
		constexpr uint16_t TAB_COUNT = 5;

		// The top-left of each list's first row; a row is 35 high, and the
		// next starts 40 below.
		constexpr Point<int16_t> STORED_ROW = Point<int16_t>(6, 87);
		constexpr Point<int16_t> CARRIED_ROW = Point<int16_t>(236, 127);
		constexpr Point<int16_t> ROW_SIZE = Point<int16_t>(199, 35);
		constexpr int16_t ROW_PITCH = 40;
		// The selection behind a row, from the row's left.
		constexpr int16_t STORED_SELECT_X = 38;
		constexpr int16_t CARRIED_SELECT_X = 37;

		// Within a row: the icon's origin, the name's two lines, the count.
		constexpr Point<int16_t> ICON = Point<int16_t>(3, 34);
		constexpr Point<int16_t> NAME = Point<int16_t>(44, 1);
		constexpr int16_t NAME_LINE = 19;
		constexpr int16_t NAME_WIDTH = 141;
		constexpr Point<int16_t> COUNT = Point<int16_t>(1, 20);

		constexpr int16_t STORED_SLIDER_X = 210;
		constexpr Range<int16_t> STORED_SLIDER = Range<int16_t>(88, 272);
		constexpr int16_t CARRIED_SLIDER_X = 441;
		constexpr Range<int16_t> CARRIED_SLIDER = Range<int16_t>(127, 272);

		// Cash items can't be stored, nor quest or untradable ones.
		bool storable(int32_t itemid)
		{
			const ItemData &item = ItemData::get(itemid);

			return item.is_valid() && !item.is_cashitem() && !item.is_questitem() && !item.is_untradable();
		}

		// Use, setup, and etc items show how many there are.
		bool shows_count(InventoryType::Id tab)
		{
			return tab == InventoryType::Id::USE || tab == InventoryType::Id::SETUP || tab == InventoryType::Id::ETC;
		}

		void scroll(int16_t &offset, int16_t size, int16_t rows, bool upwards)
		{
			int16_t next = offset + (upwards ? -1 : 1);

			if (next >= 0 && next <= size - rows)
				offset = next;
		}

		std::string with_fee(const char *format, int32_t fee)
		{
			char message[128];
			std::snprintf(message, sizeof(message), format, fee);

			return message;
		}
	}

	UIStorage::UIStorage(const Inventory &in_inventory)
			: UIDragElement<PosSTORAGE>(), inventory(in_inventory), npcid(0), put_fee(0), get_fee(0), slots(0),
			  meso(0), tab(InventoryType::Id::EQUIP), pending(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Trunk"];

		Texture bg = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		// v83 puts a button's top-left at its position, whatever the
		// canvas origin (BtSort's is (35, 9)).
		auto button = [&](const char *name, Point<int16_t> position)
		{
			nl::node bt = src[name];

			return std::make_unique<MapleButton>(bt, position + Texture(bt["normal"]["0"]).get_origin());
		};

		buttons[Buttons::EXIT] = button("BtExit", EXIT_BUTTON);
		buttons[Buttons::GET] = button("BtGet", GET_BUTTON);
		buttons[Buttons::PUT] = button("BtPut", PUT_BUTTON);
		buttons[Buttons::SORT] = button("BtSort", SORT_BUTTON);
		buttons[Buttons::OUT_COIN] = button("BtOutCoin", OUT_COIN_BUTTON);
		buttons[Buttons::IN_COIN] = button("BtInCoin", IN_COIN_BUTTON);

		tabs = ClassicTabs(ClassicTabs::TAB5, src["Tab"]["enabled"], src["Tab"]["disabled"], TAB_FILL);

		for (uint16_t i = 0; i < TAB_COUNT; i++)
			buttons[Buttons::TAB0 + i] = std::make_unique<AreaButton>(
					TABS + Point<int16_t>(tabs.pitch() * i, 0), Point<int16_t>(tabs.pitch(), tabs.height()));

		for (int16_t i = 0; i < STORED_ROWS; i++)
			buttons[Buttons::STORED0 + i] = std::make_unique<AreaButton>(
					STORED_ROW + Point<int16_t>(0, ROW_PITCH * i), ROW_SIZE);

		for (int16_t i = 0; i < CARRIED_ROWS; i++)
			buttons[Buttons::CARRIED0 + i] = std::make_unique<AreaButton>(
					CARRIED_ROW + Point<int16_t>(0, ROW_PITCH * i), ROW_SIZE);

		enabled_row = src["en"];
		selected_row = src["select"];

		stored_meso_label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);
		carried_meso_label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		stored_slider = Slider(
				Slider::Type::CLASSIC, STORED_SLIDER, STORED_SLIDER_X, STORED_ROWS, STORED_ROWS,
				[&](bool upwards)
				{
					scroll(stored.offset, static_cast<int16_t>(stored.rows.size()), STORED_ROWS, upwards);
				}
		);

		carried_slider = Slider(
				Slider::Type::CLASSIC, CARRIED_SLIDER, CARRIED_SLIDER_X, CARRIED_ROWS, CARRIED_ROWS,
				[&](bool upwards)
				{
					scroll(carried.offset, static_cast<int16_t>(carried.rows.size()), CARRIED_ROWS, upwards);
				}
		);

		active = false;
		dimension = bg.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 10);
	}

	void UIStorage::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		npc.draw(DrawArgument(position + NPC_FEET, true));

		stored_meso_label.draw(position + STORED_MESO);
		carried_meso_label.draw(position + CARRIED_MESO);

		tabs.draw(position + TABS, TAB_COUNT, tab - InventoryType::Id::EQUIP);

		draw_list(stored, STORED_ROW, STORED_ROWS, STORED_SELECT_X, slots);
		draw_list(carried, CARRIED_ROW, CARRIED_ROWS, CARRIED_SELECT_X, 0);

		stored_slider.draw(position);
		carried_slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIStorage::draw_list(const List &list, Point<int16_t> origin, int16_t rows, int16_t select_x,
							  int16_t limit) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"],
												Charset::Alignment::LEFT);

		for (int16_t i = 0; i < rows; i++)
		{
			int16_t index = list.offset + i;
			Point<int16_t> row = position + origin + Point<int16_t>(0, ROW_PITCH * i);

			// The storage marks the rows its slots fill.
			if (index == list.selection)
				selected_row.draw(row + Point<int16_t>(select_x, 0));
			else if (index < limit)
				enabled_row.draw(row);

			if (index >= static_cast<int16_t>(list.rows.size()))
				continue;

			const Row &entry = list.rows[index];
			entry.icon.draw(row + ICON);
			entry.name[0].draw(row + NAME);
			entry.name[1].draw(row + NAME + Point<int16_t>(0, NAME_LINE));

			if (shows_count(tab))
				countset.draw(std::to_string(entry.count), row + COUNT);
		}
	}

	void UIStorage::update()
	{
		std::string carried_meso = std::to_string(inventory.get_meso());
		string_format::split_number(carried_meso);
		carried_meso_label.change_text(carried_meso);
	}

	Button::State UIStorage::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		if (buttonid >= Buttons::TAB0 && buttonid <= Buttons::TAB4)
		{
			change_tab(static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + buttonid - Buttons::TAB0));

			return Button::State::NORMAL;
		}

		if (buttonid >= Buttons::STORED0 && buttonid <= Buttons::STORED4)
		{
			int16_t index = stored.offset + buttonid - Buttons::STORED0;

			if (index == stored.selection)
				take_out();
			else if (index < static_cast<int16_t>(stored.rows.size()))
				stored.selection = index;

			return Button::State::NORMAL;
		}

		if (buttonid >= Buttons::CARRIED0 && buttonid <= Buttons::CARRIED3)
		{
			int16_t index = carried.offset + buttonid - Buttons::CARRIED0;

			if (index == carried.selection)
				store();
			else if (index < static_cast<int16_t>(carried.rows.size()))
				carried.selection = index;

			return Button::State::NORMAL;
		}

		switch (buttonid)
		{
			case Buttons::EXIT:
				close();
				break;
			case Buttons::GET:
				take_out();
				break;
			case Buttons::PUT:
				store();
				break;
			case Buttons::SORT:
				if (can_request())
				{
					pending = true;
					TrunkRequestPacket(TrunkRequestPacket::Mode::SORT).dispatch();
				}

				break;
			case Buttons::OUT_COIN:
				take_meso();
				break;
			case Buttons::IN_COIN:
				save_meso();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIStorage::remove_cursor()
	{
		UIDragElement::remove_cursor();

		stored_slider.remove_cursor();
		carried_slider.remove_cursor();
	}

	Cursor::State UIStorage::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursoroffset = cursorpos - position;
		lastcursorpos = cursoroffset;

		for (Slider *slider : { &stored_slider, &carried_slider })
		{
			if (!slider->isenabled())
				continue;

			Cursor::State state = slider->send_cursor(cursoroffset, clicked);

			if (state != Cursor::State::IDLE)
			{
				clear_tooltip();

				return state;
			}
		}

		show_item(cursoroffset);

		Cursor::State ret = clicked ? Cursor::State::CLICKING : Cursor::State::IDLE;

		for (size_t i = 0; i < Buttons::NUM_BUTTONS; i++)
		{
			Button &button = *buttons[i];
			// Tabs and rows are areas, with no sound or hand of their own.
			bool real = i < Buttons::TAB0;

			if (button.is_active() && button.bounds(position).contains(cursorpos))
			{
				if (button.get_state() == Button::State::NORMAL)
				{
					if (real)
					{
						Sound(Sound::Name::BUTTONOVER).play();
						ret = Cursor::State::CANCLICK;
					}

					button.set_state(Button::State::MOUSEOVER);
				} else if (button.get_state() == Button::State::MOUSEOVER)
				{
					if (clicked)
					{
						if (i >= Buttons::TAB0 && i <= Buttons::TAB4)
							Sound(Sound::Name::TAB).play();
						else if (real)
							Sound(Sound::Name::BUTTONCLICK).play();

						button.set_state(button_pressed(static_cast<uint16_t>(i)));

						ret = Cursor::State::IDLE;
					} else if (real)
					{
						ret = Cursor::State::CANCLICK;
					}
				}
			} else if (button.get_state() == Button::State::MOUSEOVER)
			{
				button.set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	void UIStorage::send_scroll(double yoffset)
	{
		if (lastcursorpos.x() < CARRIED_ROW.x())
			stored_slider.send_scroll(yoffset);
		else
			carried_slider.send_scroll(yoffset);
	}

	void UIStorage::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIStorage::get_type() const
	{
		return TYPE;
	}

	void UIStorage::open(int32_t id, const storage_contents::Contents &contents)
	{
		npcid = id;

		std::string strid = string_format::extend_id(npcid, 7);
		nl::node src = nl::nx::npc[strid + ".img"];
		npc = src["stand"]["0"];
		put_fee = src["info"]["trunkPut"];
		get_fee = src["info"]["trunkGet"];

		for (auto &bag : bags)
			bag.clear();

		meso = 0;

		for (auto &button : buttons)
			button.second->set_state(Button::State::NORMAL);

		pending = false;
		change(contents);
		change_tab(InventoryType::Id::EQUIP);

		makeactive();
	}

	void UIStorage::change(const storage_contents::Contents &contents)
	{
		pending = false;
		slots = contents.slots;

		if (contents.meso)
			meso = *contents.meso;

		std::string stored_meso = std::to_string(meso);
		string_format::split_number(stored_meso);
		stored_meso_label.change_text(stored_meso);

		for (size_t type = 1; type < bags.size(); type++)
			if (contents.bags[type])
				bags[type] = *contents.bags[type];

		refresh_stored();
	}

	void UIStorage::answered()
	{
		pending = false;
	}

	void UIStorage::modify(InventoryType::Id type)
	{
		if (type == tab)
			refresh_carried();
	}

	void UIStorage::change_tab(InventoryType::Id newtab)
	{
		tab = newtab;

		stored.offset = 0;
		stored.selection = -1;
		carried.offset = 0;
		carried.selection = -1;

		refresh_stored();
		refresh_carried();
	}

	void UIStorage::refresh_stored()
	{
		stored.rows.clear();

		for (const storage_contents::Stored &item : bags[tab])
			stored.rows.push_back(make_row(item.itemid, item.count, 0));

		auto size = static_cast<int16_t>(stored.rows.size());
		stored.offset = std::max<int16_t>(0, std::min<int16_t>(stored.offset, size - STORED_ROWS));

		if (stored.selection >= size)
			stored.selection = -1;

		stored_slider.setrows(stored.offset, STORED_ROWS, std::max<int16_t>(size, STORED_ROWS));
	}

	void UIStorage::refresh_carried()
	{
		carried.rows.clear();

		for (int16_t slot = 1; slot <= inventory.get_slotmax(tab); slot++)
		{
			int32_t itemid = inventory.get_item_id(tab, slot);

			if (itemid && storable(itemid))
				carried.rows.push_back(make_row(itemid, inventory.get_item_count(tab, slot), slot));
		}

		auto size = static_cast<int16_t>(carried.rows.size());
		carried.offset = std::max<int16_t>(0, std::min<int16_t>(carried.offset, size - CARRIED_ROWS));

		if (carried.selection >= size)
			carried.selection = -1;

		carried_slider.setrows(carried.offset, CARRIED_ROWS, std::max<int16_t>(size, CARRIED_ROWS));
	}

	UIStorage::Row UIStorage::make_row(int32_t itemid, int16_t count, int16_t slot)
	{
		const ItemData &item = ItemData::get(itemid);

		Row row = { itemid, count, slot, item.get_icon(false), {} };

		// Whole words fill the first line; the rest go on the second.
		std::string name = item.get_name();
		std::string first;
		size_t end = 0;

		while (end < name.size())
		{
			size_t next = name.find(' ', end + 1);
			std::string candidate = name.substr(0, next);

			if (!first.empty() && Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, candidate).width() > NAME_WIDTH)
				break;

			first = candidate;
			end = next == std::string::npos ? name.size() : next;
		}

		std::string second = end < name.size() ? name.substr(end + 1) : "";

		row.name[0] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, first);
		row.name[1] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, second);

		return row;
	}

	int16_t UIStorage::row_at(Point<int16_t> cursor, Point<int16_t> origin, int16_t rows)
	{
		Point<int16_t> offset = cursor - origin;

		if (offset.x() < 0 || offset.x() >= ROW_SIZE.x() || offset.y() < 0 || offset.y() % ROW_PITCH >= ROW_SIZE.y())
			return -1;

		int16_t row = offset.y() / ROW_PITCH;

		return row < rows ? row : -1;
	}

	void UIStorage::show_item(Point<int16_t> cursor)
	{
		int16_t row = row_at(cursor, STORED_ROW, STORED_ROWS);

		if (row >= 0 && stored.offset + row < static_cast<int16_t>(stored.rows.size()))
		{
			UI::get().show_item(Tooltip::Parent::STORAGE, stored.rows[stored.offset + row].itemid);

			return;
		}

		row = row_at(cursor, CARRIED_ROW, CARRIED_ROWS);

		if (row >= 0 && carried.offset + row < static_cast<int16_t>(carried.rows.size()))
		{
			const Row &entry = carried.rows[carried.offset + row];

			if (tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::STORAGE, entry.slot);
			else
				UI::get().show_item(Tooltip::Parent::STORAGE, entry.itemid);

			return;
		}

		clear_tooltip();
	}

	void UIStorage::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::STORAGE);
	}

	bool UIStorage::can_request() const
	{
		return is_active() && !pending;
	}

	void UIStorage::take_out()
	{
		if (!can_request() || stored.selection < 0 || stored.selection >= static_cast<int16_t>(stored.rows.size()))
			return;

		auto type = static_cast<int8_t>(tab);
		auto index = static_cast<int8_t>(stored.selection);

		auto ondecide = [this, type, index](bool yes)
		{
			if (yes && can_request())
			{
				pending = true;
				TrunkRequestPacket(type, index).dispatch();
			}
		};

		if (get_fee > 0)
			UI::get().emplace<UIYesNo>(
					with_fee("Recovering the items will cost %d mesos.\\nAre you sure you want to recover this?", get_fee),
					ondecide);
		else
			UI::get().emplace<UIYesNo>("Are you sure you want to \\nrecover the item(s) from storage?", ondecide);
	}

	void UIStorage::store()
	{
		if (!can_request() || carried.selection < 0 || carried.selection >= static_cast<int16_t>(carried.rows.size()))
			return;

		const Row &row = carried.rows[carried.selection];
		int16_t slot = row.slot;
		int32_t itemid = row.itemid;
		int16_t count = row.count;
		int32_t fee = put_fee;

		auto confirm = [this, slot, itemid, fee](int32_t quantity)
		{
			auto shortqty = static_cast<int16_t>(quantity);

			auto ondecide = [this, slot, itemid, shortqty](bool yes)
			{
				if (yes && can_request())
				{
					pending = true;
					TrunkRequestPacket(slot, itemid, shortqty).dispatch();
				}
			};

			if (fee > 0)
				UI::get().emplace<UIYesNo>(
						with_fee("Storing this will cost %d mesos.\\nAre you sure you want to store this?", fee),
						ondecide);
			else
				UI::get().emplace<UIYesNo>("Are you sure you want to store the item(s)?", ondecide);
		};

		// Equips, stars, and bullets go whole; a stack asks how many, all of
		// it to begin with.
		if (tab == InventoryType::Id::EQUIP || shop_list::is_rechargeable(itemid) || count <= 1)
			confirm(count);
		else
			UI::get().emplace<UIEnterNumber>("How many will you store?", confirm, count, count);
	}

	void UIStorage::take_meso()
	{
		if (!can_request() || meso < 1)
			return;

		auto onenter = [this](int32_t amount)
		{
			if (amount > 0 && amount <= meso && can_request())
			{
				pending = true;
				TrunkRequestPacket(amount).dispatch();
			}
		};

		UI::get().emplace<UIEnterNumber>("How much will you take out?", onenter, meso, meso);
	}

	void UIStorage::save_meso()
	{
		auto carried_meso = static_cast<int32_t>(std::min<int64_t>(inventory.get_meso(), INT32_MAX));

		if (!can_request() || carried_meso < 1)
			return;

		auto onenter = [this](int32_t amount)
		{
			if (amount > 0 && amount <= inventory.get_meso() && can_request())
			{
				pending = true;
				TrunkRequestPacket(-amount).dispatch();
			}
		};

		UI::get().emplace<UIEnterNumber>("How much will you save?", onenter, carried_meso, carried_meso);
	}

	void UIStorage::close()
	{
		clear_tooltip();

		deactivate();
		TrunkRequestPacket(TrunkRequestPacket::Mode::CLOSE).dispatch();
	}
}
