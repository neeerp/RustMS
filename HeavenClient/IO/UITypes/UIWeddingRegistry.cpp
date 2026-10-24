#include "UIWeddingRegistry.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Audio/Audio.h"
#include "../Character/Inventory/ItemDrop.h"
#include "../Data/ItemData.h"
#include "../Gameplay/Stage.h"

#include "../Net/Packets/MarriagePackets.h"
#include "../../Constants.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The v83 windows' layout (0x9da789, 0x9dab15). Buttons put their
		// top-left at their position, whatever the canvas origin.
		constexpr Point<int16_t> GIVE_EXIT = Point<int16_t>(304, 265);
		constexpr Point<int16_t> RECEIVE_EXIT = Point<int16_t>(154, 265);
		constexpr Point<int16_t> ACTION_BUTTON = Point<int16_t>(384, 265);
		constexpr Point<int16_t> TABS = Point<int16_t>(235, 65);
		constexpr int16_t TAB_FILL = 32;
		constexpr uint16_t TAB_COUNT = 5;

		// The wishlist's lines, 18 apart.
		constexpr Point<int16_t> WISH = Point<int16_t>(11, 62);
		constexpr int16_t WISH_PITCH = 18;
		constexpr Range<int16_t> WISH_SLIDER = Range<int16_t>(60, 150);

		// The top-left of each list's first row; a row is 35 high, and the
		// next starts 40 below. The Registry lists two gifts under its
		// wishlist, the gift window five.
		constexpr Point<int16_t> GIVEN_ROW = Point<int16_t>(6, 198);
		constexpr Point<int16_t> RECEIVED_ROW = Point<int16_t>(6, 60);
		constexpr int16_t GIVEN_ROWS = 2;
		constexpr int16_t RECEIVED_ROWS = 5;
		constexpr Point<int16_t> ITEM_ROW = Point<int16_t>(236, 100);
		constexpr Point<int16_t> ROW_SIZE = Point<int16_t>(199, 35);
		constexpr int16_t ROW_PITCH = 40;
		// The selection behind a row, from the row's left.
		constexpr int16_t SELECT_X = 38;

		// Within a row: the icon's origin, the name's two lines, the count.
		constexpr Point<int16_t> ICON = Point<int16_t>(3, 34);
		constexpr Point<int16_t> NAME = Point<int16_t>(44, 1);
		constexpr int16_t NAME_LINE = 19;
		constexpr int16_t NAME_WIDTH = 141;
		constexpr Point<int16_t> COUNT = Point<int16_t>(1, 20);

		constexpr int16_t LEFT_SLIDER_X = 210;
		constexpr Range<int16_t> GIVEN_SLIDER = Range<int16_t>(197, 272);
		constexpr Range<int16_t> RECEIVED_SLIDER = Range<int16_t>(60, 255);
		constexpr int16_t ITEM_SLIDER_X = 441;
		constexpr Range<int16_t> ITEM_SLIDER = Range<int16_t>(99, 255);

		// Neither cash, quest, untradeable nor account items can be given
		// (0x9d877b).
		bool giftable(const Inventory &inventory, InventoryType::Id tab, int16_t slot, int32_t itemid)
		{
			const ItemData &item = ItemData::get(itemid);

			if (!item.is_valid())
				return false;

			int16_t flags = 0;

			if (Optional<const Equip> equip = inventory.get_equip(tab, slot))
				flags = equip->get_flags();
			else if (Optional<const Item> bundle = inventory.get_item(tab, slot))
				flags = bundle->get_flags();

			bool cash = item.is_cashitem() || inventory.get_cash_id(tab, slot) != 0;

			return item_drop::giftable(item_drop::traits(item.get_properties(), tab == InventoryType::Id::EQUIP, cash,
				flags, item_expiry::BEFORE));
		}

		// Use, setup, and etc items show how many there are.
		bool shows_count(int32_t itemid)
		{
			int32_t type = itemid / 1000000;

			return type >= 2 && type <= 4;
		}

		void scroll(int16_t &offset, int16_t size, int16_t rows, bool upwards)
		{
			int16_t next = offset + (upwards ? -1 : 1);

			if (next >= 0 && next <= size - rows)
				offset = next;
		}
	}

	UIWeddingRegistry::UIWeddingRegistry(Kind k, bool female)
			: UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), kind(k),
			  inventory(Stage::get().get_player().get_inventory()), wish_offset(0),
			  taking(InventoryType::Id::NONE), tab(InventoryType::Id::EQUIP), pending(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Wedding"]["wishList"];
		bool give = kind == Kind::GIVE;
		nl::node window = src[give ? "give" : "receive"];

		// The Registry is always the groom's; the gift window is the player's.
		background = window[give || !female ? "groom" : "bride"];
		selected_row = src["give"]["select"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
			(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		auto button = [&](const char *name, Point<int16_t> at)
		{
			nl::node bt = window[name];

			return std::make_unique<MapleButton>(bt, at + Texture(bt["normal"]["0"]).get_origin());
		};

		buttons[Buttons::EXIT] = button("BtExit", give ? GIVE_EXIT : RECEIVE_EXIT);
		buttons[Buttons::ACTION] = button(give ? "BtPut" : "BtGet", ACTION_BUTTON);

		tabs = ClassicTabs(ClassicTabs::TAB5, window["Tab"]["En"], window["Tab"]["Ds"], TAB_FILL);

		for (uint16_t i = 0; i < TAB_COUNT; i++)
			buttons[Buttons::TAB0 + i] = std::make_unique<AreaButton>(
					TABS + Point<int16_t>(tabs.pitch() * i, 0), Point<int16_t>(tabs.pitch(), tabs.height()));

		Point<int16_t> given_row = give ? GIVEN_ROW : RECEIVED_ROW;
		int16_t given_rows = give ? GIVEN_ROWS : RECEIVED_ROWS;

		for (int16_t i = 0; i < given_rows; i++)
			buttons[Buttons::GIFT0 + i] = std::make_unique<AreaButton>(
					given_row + Point<int16_t>(0, ROW_PITCH * i), ROW_SIZE);

		for (int16_t i = 0; i < ITEM_ROWS; i++)
			buttons[Buttons::ITEM0 + i] = std::make_unique<AreaButton>(
					ITEM_ROW + Point<int16_t>(0, ROW_PITCH * i), ROW_SIZE);

		if (give)
			wish_slider = Slider(
					Slider::Type::CLASSIC, WISH_SLIDER, LEFT_SLIDER_X, WISH_ROWS, WISH_ROWS,
					[&](bool upwards)
					{
						scroll(wish_offset, static_cast<int16_t>(wishes.size()), WISH_ROWS, upwards);
					}
			);

		gift_slider = Slider(
				Slider::Type::CLASSIC, give ? GIVEN_SLIDER : RECEIVED_SLIDER, LEFT_SLIDER_X, given_rows, given_rows,
				[&, given_rows](bool upwards)
				{
					scroll(given.offset, static_cast<int16_t>(given.rows.size()), given_rows, upwards);
				}
		);

		item_slider = Slider(
				Slider::Type::CLASSIC, ITEM_SLIDER, ITEM_SLIDER_X, ITEM_ROWS, ITEM_ROWS,
				[&](bool upwards)
				{
					scroll(carried.offset, static_cast<int16_t>(carried.rows.size()), ITEM_ROWS, upwards);
				}
		);

		change_tab(InventoryType::Id::EQUIP);
		refresh_gifts();
	}

	void UIWeddingRegistry::draw(float alpha) const
	{
		// The art is centred on its origin.
		background.draw(position + background.get_origin());

		tabs.draw(position + TABS, TAB_COUNT, tab - InventoryType::Id::EQUIP);

		if (kind == Kind::GIVE)
		{
			for (int16_t i = 0; i < WISH_ROWS && wish_offset + i < static_cast<int16_t>(wishes.size()); i++)
				wishes[wish_offset + i].draw(position + WISH + Point<int16_t>(0, WISH_PITCH * i));

			wish_slider.draw(position);
			draw_list(given, GIVEN_ROW, GIVEN_ROWS);
		}
		else
		{
			draw_list(given, RECEIVED_ROW, RECEIVED_ROWS);
		}

		draw_list(carried, ITEM_ROW, ITEM_ROWS);

		gift_slider.draw(position);
		item_slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIWeddingRegistry::draw_list(const List &list, Point<int16_t> origin, int16_t rows) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"],
												Charset::Alignment::LEFT);

		for (int16_t i = 0; i < rows; i++)
		{
			int16_t index = list.offset + i;
			Point<int16_t> row = position + origin + Point<int16_t>(0, ROW_PITCH * i);

			if (index == list.selection)
				selected_row.draw(row + Point<int16_t>(SELECT_X, 0));

			if (index >= static_cast<int16_t>(list.rows.size()))
				continue;

			const Row &entry = list.rows[index];
			entry.icon.draw(row + ICON);
			entry.name[0].draw(row + NAME);
			entry.name[1].draw(row + NAME + Point<int16_t>(0, NAME_LINE));

			if (shows_count(entry.itemid))
				countset.draw(std::to_string(entry.count), row + COUNT);
		}
	}

	Button::State UIWeddingRegistry::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		if (buttonid >= Buttons::TAB0 && buttonid <= Buttons::TAB4)
		{
			auto newtab = static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + buttonid - Buttons::TAB0);

			// The tab shown, clicked again, drops the pick but stays where
			// it was (0x9d89ad).
			if (newtab == tab)
			{
				carried.selection = -1;
				refresh_items();
			} else
			{
				change_tab(newtab);
			}

			return Button::State::NORMAL;
		}

		// Selecting a row in one list clears the other's.
		if (buttonid >= Buttons::GIFT0 && buttonid <= Buttons::GIFT4)
		{
			int16_t index = given.offset + buttonid - Buttons::GIFT0;

			if (index < static_cast<int16_t>(given.rows.size()))
			{
				given.selection = index;
				carried.selection = -1;
			}

			return Button::State::NORMAL;
		}

		if (buttonid >= Buttons::ITEM0 && buttonid <= Buttons::ITEM3)
		{
			int16_t index = carried.offset + buttonid - Buttons::ITEM0;

			if (index < static_cast<int16_t>(carried.rows.size()))
			{
				carried.selection = index;
				given.selection = -1;
			}

			return Button::State::NORMAL;
		}

		switch (buttonid)
		{
			case Buttons::EXIT:
				close();
				break;
			case Buttons::ACTION:
				if (kind == Kind::GIVE)
					give();
				else
					take();

				break;
		}

		return Button::State::NORMAL;
	}

	// A double click on a row, which its first click selected, acts on it.
	void UIWeddingRegistry::doubleclick(Point<int16_t> cursorpos)
	{
		for (uint16_t id = Buttons::GIFT0; id <= Buttons::ITEM3; id++)
		{
			auto button = buttons.find(id);

			if (button == buttons.end() || !button->second->is_active()
				|| !button->second->bounds(position).contains(cursorpos))
				continue;

			if (id <= Buttons::GIFT4)
			{
				if (kind == Kind::RECEIVE && given.offset + id - Buttons::GIFT0 == given.selection)
					take();
			}
			else if (kind == Kind::GIVE && carried.offset + id - Buttons::ITEM0 == carried.selection)
			{
				give();
			}

			return;
		}
	}

	void UIWeddingRegistry::remove_cursor()
	{
		UIElement::remove_cursor();

		wish_slider.remove_cursor();
		gift_slider.remove_cursor();
		item_slider.remove_cursor();
	}

	Cursor::State UIWeddingRegistry::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursoroffset = cursorpos - position;
		lastcursorpos = cursoroffset;

		// A button held keeps the cursor, sliders and all, until the mouse
		// comes back up.
		if (is_holding_button())
			return UIElement::send_cursor(clicked, cursorpos);

		for (Slider *slider : { &wish_slider, &gift_slider, &item_slider })
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

		// Exit and Put or Get click as the mouse comes back up over them.
		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// The one held keeps the cursor until the mouse comes back up.
		if (is_holding_button())
			return ret;

		// Tabs and rows are areas, with no sound or hand of their own, that
		// v83 picks as the mouse goes down, not as it moves held down. A tab
		// clicked again rebuilds its list.
		for (size_t i = Buttons::TAB0; i < Buttons::NUM_BUTTONS; i++)
		{
			if (!buttons.count(static_cast<uint16_t>(i)))
				continue;

			Button &button = *buttons[i];

			if (button.is_active() && button.bounds(position).contains(cursorpos))
			{
				if (button.get_state() == Button::State::NORMAL)
					button.set_state(Button::State::MOUSEOVER);

				if (button.get_state() == Button::State::MOUSEOVER && clicked && UI::get().is_cursor_press())
				{
					if (i <= Buttons::TAB4)
						Sound(Sound::Name::TAB).play();

					button.set_state(button_pressed(static_cast<uint16_t>(i)));

					ret = Cursor::State::IDLE;
				}
			} else if (button.get_state() == Button::State::MOUSEOVER)
			{
				button.set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	void UIWeddingRegistry::send_scroll(double yoffset)
	{
		const Point<int16_t> &cursor = lastcursorpos;

		if (cursor.x() >= ITEM_ROW.x())
			item_slider.send_scroll(yoffset);
		else if (kind == Kind::GIVE && cursor.y() < GIVEN_ROW.y())
			wish_slider.send_scroll(yoffset);
		else
			gift_slider.send_scroll(yoffset);
	}

	// Escape and Enter close it, as v83's dialogs do.
	void UIWeddingRegistry::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	UIElement::Type UIWeddingRegistry::get_type() const
	{
		return TYPE;
	}

	UIWeddingRegistry::Kind UIWeddingRegistry::get_kind() const
	{
		return kind;
	}

	void UIWeddingRegistry::show_wishes(const std::vector<std::string> &list)
	{
		wishes.clear();

		for (const std::string &wish : list)
			wishes.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, wish);

		auto size = static_cast<int16_t>(wishes.size());
		wish_offset = std::max<int16_t>(0, std::min<int16_t>(wish_offset, size - WISH_ROWS));
		wish_slider.setrows(wish_offset, WISH_ROWS, std::max<int16_t>(size, WISH_ROWS));
	}

	void UIWeddingRegistry::read_gifts(InPacket &recv)
	{
		wedding_gifts::read_gifts(recv, gifts);
		refresh_gifts();
	}

	void UIWeddingRegistry::sent()
	{
		close();

		UI::get().emplace<UIOk>("You have sent a gift.", [](bool) {});
	}

	void UIWeddingRegistry::taken()
	{
		pending = false;

		if (taking != InventoryType::Id::NONE)
			change_tab(taking);
	}

	void UIWeddingRegistry::answered()
	{
		pending = false;
	}

	void UIWeddingRegistry::modify(InventoryType::Id type)
	{
		if (type == tab)
			refresh_items();
	}

	void UIWeddingRegistry::change_tab(InventoryType::Id newtab)
	{
		tab = newtab;

		carried.offset = 0;
		carried.selection = -1;

		refresh_items();
	}

	void UIWeddingRegistry::refresh_gifts()
	{
		given.rows.clear();

		for (size_t type = 1; type < gifts.size(); type++)
			for (size_t index = 0; index < gifts[type].size(); index++)
				given.rows.push_back(make_row(gifts[type][index].itemid, gifts[type][index].count,
					static_cast<int16_t>(index)));

		int16_t rows = kind == Kind::GIVE ? GIVEN_ROWS : RECEIVED_ROWS;
		auto size = static_cast<int16_t>(given.rows.size());
		given.offset = std::max<int16_t>(0, std::min<int16_t>(given.offset, size - rows));
		given.selection = -1;

		gift_slider.setrows(given.offset, rows, std::max<int16_t>(size, rows));
	}

	void UIWeddingRegistry::refresh_items()
	{
		// The row selected before stays so if it is still there.
		int32_t selected_id = 0;
		int16_t selected_slot = 0;

		if (carried.selection >= 0 && carried.selection < static_cast<int16_t>(carried.rows.size()))
		{
			selected_id = carried.rows[carried.selection].itemid;
			selected_slot = carried.rows[carried.selection].slot;
		}

		carried.rows.clear();
		carried.selection = -1;

		for (int16_t slot = 1; slot <= inventory.get_slotmax(tab); slot++)
		{
			int32_t itemid = inventory.get_item_id(tab, slot);

			if (!itemid || !giftable(inventory, tab, slot, itemid))
				continue;

			if (itemid == selected_id && slot == selected_slot)
				carried.selection = static_cast<int16_t>(carried.rows.size());

			carried.rows.push_back(make_row(itemid, inventory.get_item_count(tab, slot), slot));
		}

		auto size = static_cast<int16_t>(carried.rows.size());
		carried.offset = std::max<int16_t>(0, std::min<int16_t>(carried.offset, size - ITEM_ROWS));

		item_slider.setrows(carried.offset, ITEM_ROWS, std::max<int16_t>(size, ITEM_ROWS));
	}

	UIWeddingRegistry::Row UIWeddingRegistry::make_row(int32_t itemid, int16_t count, int16_t slot)
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

	int16_t UIWeddingRegistry::row_at(Point<int16_t> cursor, Point<int16_t> origin, int16_t rows)
	{
		Point<int16_t> offset = cursor - origin;

		if (offset.x() < 0 || offset.x() >= ROW_SIZE.x() || offset.y() < 0 || offset.y() % ROW_PITCH >= ROW_SIZE.y())
			return -1;

		int16_t row = offset.y() / ROW_PITCH;

		return row < rows ? row : -1;
	}

	void UIWeddingRegistry::show_item(Point<int16_t> cursor)
	{
		bool give = kind == Kind::GIVE;
		int16_t row = row_at(cursor, give ? GIVEN_ROW : RECEIVED_ROW, give ? GIVEN_ROWS : RECEIVED_ROWS);

		if (row >= 0 && given.offset + row < static_cast<int16_t>(given.rows.size()))
		{
			const Row &entry = given.rows[given.offset + row];
			const std::vector<storage_contents::Stored> &bag = gifts[entry.itemid / 1000000 % gifts.size()];

			if (entry.slot < static_cast<int16_t>(bag.size()))
			{
				UI::get().show_item(Tooltip::Parent::WEDDINGREGISTRY, bag[entry.slot]);

				return;
			}
		}

		row = row_at(cursor, ITEM_ROW, ITEM_ROWS);

		if (row >= 0 && carried.offset + row < static_cast<int16_t>(carried.rows.size()))
		{
			const Row &entry = carried.rows[carried.offset + row];

			if (tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::WEDDINGREGISTRY, entry.slot);
			else if (auto item = inventory.get_item(tab, entry.slot))
				UI::get().show_item(Tooltip::Parent::WEDDINGREGISTRY, entry.itemid, item->get_expiration(),
					item->get_flags(), true, 0, item->get_owner());
			else
				UI::get().show_item(Tooltip::Parent::WEDDINGREGISTRY, entry.itemid);

			return;
		}

		clear_tooltip();
	}

	void UIWeddingRegistry::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::WEDDINGREGISTRY);
	}

	bool UIWeddingRegistry::can_request() const
	{
		return is_active() && !pending;
	}

	void UIWeddingRegistry::give()
	{
		if (!can_request() || carried.selection < 0 || carried.selection >= static_cast<int16_t>(carried.rows.size()))
			return;

		// One gift for each wishlist (0x9da382).
		if (!given.rows.empty())
		{
			UI::get().emplace<UIOk>("You cannot give more than one present for each wishlist.", [](bool) {});

			return;
		}

		const Row &row = carried.rows[carried.selection];
		int16_t slot = row.slot;
		int32_t itemid = row.itemid;
		int16_t count = row.count;
		InventoryType::Id type = tab;
		bool expires = false;

		if (Optional<const Item> item = inventory.get_item(tab, slot))
			expires = item->get_expiration() < item_expiry::BEFORE;

		auto confirm = [this, type, slot, itemid](int32_t quantity)
		{
			auto shortqty = static_cast<int16_t>(quantity);

			UI::get().emplace<UIYesNo>("Do you want to gift this item?",
				[this, type, slot, itemid, shortqty](bool yes)
				{
					// It must still be where it was asked about.
					if (yes && can_request() && inventory.get_item_id(type, slot) == itemid)
					{
						pending = true;
						WeddingActionPacket(slot, itemid, shortqty).dispatch();
					}
				});
		};

		// Equips, stars, bullets and anything that expires go whole, as one;
		// a stack asks how many, all of it to begin with.
		if (item_drop::asks_count(itemid, count, expires))
			UI::get().emplace<UIEnterNumber>("How many will you store?", confirm, count, count);
		else
			confirm(1);
	}

	void UIWeddingRegistry::take()
	{
		if (!can_request() || given.selection < 0 || given.selection >= static_cast<int16_t>(given.rows.size()))
			return;

		const Row &row = given.rows[given.selection];
		auto type = static_cast<uint8_t>(row.itemid / 1000000);
		auto index = static_cast<uint8_t>(row.slot);

		UI::get().emplace<UIYesNo>("Are you sure you want to \\nrecover the item(s) from storage?",
			[this, type, index](bool yes)
			{
				if (yes && can_request())
				{
					pending = true;
					taking = static_cast<InventoryType::Id>(type);
					WeddingActionPacket(type, index).dispatch();
				}
			});
	}

	void UIWeddingRegistry::close()
	{
		clear_tooltip();

		deactivate();
		WeddingActionPacket().dispatch();
	}
}
