//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UIMonsterBook.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Data/ItemData.h"
#include "../../Graphics/Geometry.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/PlayerPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cctype>

namespace ms
{
	namespace
	{
		// The window, measured from period footage.
		constexpr Point<int16_t> CLOSE = Point<int16_t>(429, 8);
		constexpr Point<int16_t> CARD_SLOT = Point<int16_t>(39, 25);
		constexpr Point<int16_t> SEARCH = Point<int16_t>(174, 29);
		constexpr Point<int16_t> SEARCH_FIELD_LT = Point<int16_t>(50, 30);
		constexpr Point<int16_t> SEARCH_FIELD_RB = Point<int16_t>(166, 44);
		constexpr Point<int16_t> PREV = Point<int16_t>(51, 287);
		constexpr Point<int16_t> NEXT = Point<int16_t>(184, 287);
		constexpr Point<int16_t> INFO_PREV = Point<int16_t>(271, 287);
		constexpr Point<int16_t> INFO_NEXT = Point<int16_t>(408, 287);
		// Tabs stick out of the book: the summary and the categories on the
		// left, the monster's pages on the right. A selected tab is wider
		// and drawn further in.
		constexpr Point<int16_t> INFO_TAB = Point<int16_t>(-10, 23);
		constexpr Point<int16_t> INFO_TAB_SELECTED = Point<int16_t>(-3, 25);
		constexpr Point<int16_t> LEFT_TAB = Point<int16_t>(-8, 126);
		constexpr Point<int16_t> LEFT_TAB_SELECTED = Point<int16_t>(-3, 126);
		constexpr Point<int16_t> LEFT_TAB_SIZE = Point<int16_t>(20, 20);
		constexpr Point<int16_t> RIGHT_TAB = Point<int16_t>(457, 26);
		constexpr Point<int16_t> RIGHT_TAB_SELECTED = Point<int16_t>(439, 26);
		constexpr Point<int16_t> RIGHT_TAB_SIZE = Point<int16_t>(51, 37);
		constexpr int16_t RIGHT_TAB_PITCH = 38;
		// How far the tabs reach out of the background.
		constexpr int16_t LEFT_REACH = 8;
		constexpr int16_t RIGHT_REACH = 502;
		// The first card's top-left; cards are 27x38.
		constexpr Point<int16_t> CARD = Point<int16_t>(47, 56);
		constexpr Point<int16_t> CARD_PITCH = Point<int16_t>(33, 45);
		constexpr Point<int16_t> CARD_SIZE = Point<int16_t>(27, 38);
		constexpr Point<int16_t> CARD_COUNT = Point<int16_t>(1, 27);
		// The summary page.
		constexpr Point<int16_t> INFO_PAGE = Point<int16_t>(39, 29);
		constexpr Point<int16_t> BOOK_ICON = Point<int16_t>(116, 42);
		constexpr int16_t INFO_VALUE_X = 180;
		constexpr int16_t MASTER_LEVEL_Y = 24;
		constexpr int16_t TOTAL_CARDS_Y = 76;
		constexpr int16_t SPECIAL_CARDS_Y = 108;
		constexpr int16_t BASIC_CARDS_Y = 128;
		constexpr Point<int16_t> COVER_CARD = Point<int16_t>(96, 159);
		// The right page.
		constexpr Point<int16_t> MOB_NAME = Point<int16_t>(354, 23);
		constexpr Point<int16_t> MOB = Point<int16_t>(343, 170);
		constexpr Point<int16_t> MOB_HP = Point<int16_t>(304, 288);
		constexpr Point<int16_t> MOB_MP = Point<int16_t>(355, 288);
		constexpr Point<int16_t> TEXT = Point<int16_t>(269, 60);
		constexpr int16_t TEXT_WIDTH = 154;
		constexpr int16_t TEXT_HEIGHT = 205;
		constexpr Point<int16_t> MAPS = Point<int16_t>(269, 58);
		constexpr int16_t MAP_LINE = 20;
		constexpr int16_t MAP_LINES = 11;
		constexpr int16_t MAP_INDENT = 2;
		constexpr Point<int16_t> REWARD = Point<int16_t>(280, 80);
		constexpr Point<int16_t> REWARD_PITCH = Point<int16_t>(36, 35);
		constexpr int16_t REWARD_COLUMNS = 4;
		constexpr int16_t REWARD_ROWS = 5;
		constexpr int16_t ICON_SIZE = 32;
		// The cover menu's two rows.
		constexpr int16_t MENU_ROW = 14;
		constexpr int16_t MENU_WIDTH = 85;
		// Missing cards are drawn in faint grey.
		constexpr float MISSING_OPACITY = 0.5f;

		// Split text into pages that fit the right page.
		std::vector<Text> paginate(const std::string &text)
		{
			std::vector<Text> pages;
			std::string page;
			size_t first = 0;

			while (first < text.size())
			{
				size_t last = text.find(' ', first);

				if (last == std::string::npos)
					last = text.size();

				std::string word = text.substr(first, last - first);
				first = last + 1;

				if (word.empty())
					continue;

				std::string longer = page.empty() ? word : page + " " + word;
				Text measured = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, longer, TEXT_WIDTH);

				if (!page.empty() && measured.height() > TEXT_HEIGHT)
				{
					pages.emplace_back(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, page, TEXT_WIDTH);

					// A page doesn't start with blank lines.
					while (word.compare(0, 2, "\\n") == 0)
						word.erase(0, 2);

					page = word;
				} else
				{
					page = longer;
				}
			}

			if (!page.empty())
				pages.emplace_back(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, page, TEXT_WIDTH);

			return pages;
		}

		std::string lowercase(std::string text)
		{
			std::transform(text.begin(), text.end(), text.begin(),
						   [](unsigned char c)
						   {
							   return std::tolower(c);
						   });

			return text;
		}

		// The values of a list whose children are named 0, 1, 2, ..., in
		// numeric order.
		std::vector<int32_t> numbered(nl::node list)
		{
			std::vector<std::pair<int32_t, int32_t>> entries;

			for (nl::node entry : list)
				entries.emplace_back(std::stoi(entry.name()), entry);

			std::sort(entries.begin(), entries.end());

			std::vector<int32_t> values;

			for (auto &entry : entries)
				values.push_back(entry.second);

			return values;
		}

		std::string mob_name_of(int32_t mob)
		{
			return nl::nx::string["Mob.img"][std::to_string(mob)]["name"];
		}
	}

	const std::vector<std::vector<UIMonsterBook::Card>> &UIMonsterBook::catalogue()
	{
		static const std::vector<std::vector<Card>> cards = []()
		{
			std::vector<std::vector<Card>> by_tab(TABS);

			for (nl::node card : nl::nx::item["Consume"]["0238.img"])
			{
				int32_t id = std::stoi(card.name());
				int32_t mob = card["info"]["mob"];
				size_t category = id / 1000 % 10;

				if (category < TABS)
					by_tab[category].push_back({ id, mob });
			}

			for (auto &cards : by_tab)
				std::sort(cards.begin(), cards.end(),
						  [](const Card &a, const Card &b)
						  {
							  return a.id < b.id;
						  });

			return by_tab;
		}();

		return cards;
	}

	UIMonsterBook::UIMonsterBook(const MonsterBook &in_book)
			: UIDragElement<PosMONSTERBOOK>(), book(in_book), tab(SUMMARY), page(0), selected(0), info(Info::BASIC),
			  info_page(0), menu_card(0), menu_click(false)
	{
		nl::node MonsterBook = NxFiles::ui_classic["UIWindow.img"]["MonsterBook"];
		nl::node backgrnd = MonsterBook["backgrnd"];

		sprites.emplace_back(backgrnd);

		cardslot = MonsterBook["cardSlot"];
		infopage = MonsterBook["infoPage"];
		select_frame = MonsterBook["select"];
		cover_frame = MonsterBook["cover"];
		fullmark = MonsterBook["fullMark"];

		for (nl::node icon : MonsterBook["icon"])
			book_icons.emplace_back(icon);

		for (uint16_t i = 0; i < TABS; i++)
		{
			nl::node lefttab = MonsterBook["LeftTab"][i];

			lefttabs[false].emplace_back(lefttab["normal"]["0"]);
			lefttabs[true].emplace_back(lefttab["selected"]["0"]);

			Point<int16_t> tab_pos = LEFT_TAB + Point<int16_t>(0, LEFT_TAB_SIZE.y() * i);
			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos, LEFT_TAB_SIZE);
			buttons[Buttons::BT_TAB0 + i]->set_tab();
		}

		infotab[false] = MonsterBook["LeftTabInfo"]["0"]["normal"]["0"];
		infotab[true] = MonsterBook["LeftTabInfo"]["0"]["selected"]["0"];
		buttons[Buttons::BT_INFO] = std::make_unique<AreaButton>(INFO_TAB, infotab[false].get_dimensions());

		for (uint16_t i = 0; i < Info::NUM_INFO; i++)
		{
			nl::node righttab = MonsterBook["RightTab"][i];

			righttabs[0].emplace_back(righttab["normal"]["0"]);
			righttabs[1].emplace_back(righttab["selected"]["0"]);
			righttabs[2].emplace_back(righttab["disabled"]["0"]);

			Point<int16_t> tab_pos = RIGHT_TAB + Point<int16_t>(0, RIGHT_TAB_PITCH * i);
			buttons[Buttons::BT_RIGHTTAB0 + i] = std::make_unique<AreaButton>(tab_pos, RIGHT_TAB_SIZE);
			buttons[Buttons::BT_RIGHTTAB0 + i]->set_tab();
		}

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(MonsterBook["BtClose"], CLOSE);
		buttons[Buttons::BT_SEARCH] = std::make_unique<MapleButton>(MonsterBook["BtSearch"], SEARCH);
		buttons[Buttons::BT_PREV] = std::make_unique<MapleButton>(MonsterBook["arrowLeft"], PREV);
		buttons[Buttons::BT_NEXT] = std::make_unique<MapleButton>(MonsterBook["arrowRight"], NEXT);
		buttons[Buttons::BT_INFO_PREV] = std::make_unique<MapleButton>(MonsterBook["arrowLeft"], INFO_PREV);
		buttons[Buttons::BT_INFO_NEXT] = std::make_unique<MapleButton>(MonsterBook["arrowRight"], INFO_NEXT);
		buttons[Buttons::BT_REGISTER] = std::make_unique<MapleButton>(MonsterBook["ContextMenu"]["BtRegister"]);
		buttons[Buttons::BT_RELEASE] = std::make_unique<MapleButton>(MonsterBook["ContextMenu"]["BtRelease"]);
		buttons[Buttons::BT_REGISTER]->set_active(false);
		buttons[Buttons::BT_RELEASE]->set_active(false);

		countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		searchfield = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
								Rectangle<int16_t>(SEARCH_FIELD_LT, SEARCH_FIELD_RB), 12);
		searchfield.set_enter_callback(
				[&](std::string text)
				{
					search(text);
				}
		);

		master_level = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::DARKGREY);
		total_cards = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::DARKGREY);
		special_cards = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::DARKGREY);
		basic_cards = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::DARKGREY);

		mob_name = Text(Text::Font::A12B, Text::Alignment::CENTER, Color::Name::DARKGREY);
		mob_hp = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::RED);
		mob_mp = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLUE);

		dimension = Texture(backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		change_tab(SUMMARY);
		update_cards();
	}

	void UIMonsterBook::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		// The tabs of the left page.
		for (uint16_t i = 0; i < TABS; i++)
		{
			bool chosen = i == tab;
			Point<int16_t> tab_pos = (chosen ? LEFT_TAB_SELECTED : LEFT_TAB) + Point<int16_t>(0, LEFT_TAB_SIZE.y() * i);

			lefttabs[chosen][i].draw(position + tab_pos);
		}

		infotab[tab == SUMMARY].draw(position + (tab == SUMMARY ? INFO_TAB_SELECTED : INFO_TAB));

		// The tabs of the right page.
		for (uint16_t i = 0; i < Info::NUM_INFO; i++)
		{
			Point<int16_t> tab_pos = Point<int16_t>(0, RIGHT_TAB_PITCH * i);

			if (selected && i == info)
				righttabs[1][i].draw(position + RIGHT_TAB_SELECTED + tab_pos);
			else if (selected && info_unlocked(i))
				righttabs[0][i].draw(position + RIGHT_TAB + tab_pos);
			else
				righttabs[2][i].draw(position + RIGHT_TAB + tab_pos);
		}

		if (tab == SUMMARY)
		{
			if (!book_icons.empty())
			{
				size_t icon = std::clamp<int16_t>(book.get_book_level() - 1, 0, book_icons.size() - 1);
				book_icons[icon].draw(position + BOOK_ICON);
			}

			infopage.draw(position + INFO_PAGE);
			master_level.draw(position + Point<int16_t>(INFO_VALUE_X, MASTER_LEVEL_Y));
			total_cards.draw(position + Point<int16_t>(INFO_VALUE_X, TOTAL_CARDS_Y));
			special_cards.draw(position + Point<int16_t>(INFO_VALUE_X, SPECIAL_CARDS_Y));
			basic_cards.draw(position + Point<int16_t>(INFO_VALUE_X, BASIC_CARDS_Y));

			if (int32_t cover = book.get_cover())
				ItemData::get(cover).get_icon(true).draw(position + COVER_CARD + Point<int16_t>(0, CARD_SIZE.y()));
		} else
		{
			cardslot.draw(position + CARD_SLOT);
			searchfield.draw(position);

			const std::vector<Card> &cards = catalogue()[tab];

			for (int16_t i = 0; i < PER_PAGE; i++)
			{
				size_t index = page * PER_PAGE + i;

				if (index >= cards.size())
					break;

				int32_t card = cards[index].id;
				int8_t level = book.get_level(card);
				Point<int16_t> card_pos = position + CARD + Point<int16_t>(CARD_PITCH.x() * (i % COLUMNS),
																		   CARD_PITCH.y() * (i / COLUMNS));
				Point<int16_t> card_origin = card_pos + Point<int16_t>(0, CARD_SIZE.y());
				const Texture &icon = ItemData::get(card).get_icon(true);

				if (level > 0)
				{
					icon.draw(card_origin);

					if (level >= MonsterBook::MAX_LEVEL)
						fullmark.draw(card_pos + CARD_COUNT);
					else
						countset.draw(std::to_string(level), card_pos + CARD_COUNT);
				} else
				{
					icon.draw(DrawArgument(card_origin, Color::grayscale(MISSING_OPACITY)));
				}

				if (card == book.get_cover())
					cover_frame.draw(card_origin);

				if (card == selected)
					select_frame.draw(card_origin);
			}
		}

		if (selected)
		{
			int8_t level = book.get_level(selected);

			mob_name.draw(position + MOB_NAME);

			switch (info)
			{
				case Info::BASIC:
					if (level > 0)
						mob_stand.draw(position + MOB, 1.0f);

					mob_hp.draw(position + MOB_HP);
					mob_mp.draw(position + MOB_MP);
					break;
				case Info::EPISODE:
					if (info_page < episode_pages.size())
						episode_pages[info_page].draw(position + TEXT);

					break;
				case Info::DROPPING:
				{
					int16_t per_page = REWARD_COLUMNS * REWARD_ROWS;

					for (int16_t i = 0; i < per_page; i++)
					{
						size_t index = info_page * per_page + i;

						if (index >= rewards.size())
							break;

						Point<int16_t> reward_pos = REWARD + Point<int16_t>(REWARD_PITCH.x() * (i % REWARD_COLUMNS),
																			REWARD_PITCH.y() * (i / REWARD_COLUMNS));

						// Each item sits in a pale slot with a grey edge, its
						// corners left open so it looks rounded.
						Point<int16_t> slot = position + reward_pos - Point<int16_t>(1, 1);
						int16_t edge = ICON_SIZE + 2;

						ColorBox(ICON_SIZE, ICON_SIZE, Color::Name::WHITE, 0.5f).draw(slot + Point<int16_t>(1, 1));
						ColorBox(ICON_SIZE, 1, Color::Name::SILVERCHALICE, 1.0f).draw(slot + Point<int16_t>(1, 0));
						ColorBox(ICON_SIZE, 1, Color::Name::SILVERCHALICE, 1.0f).draw(slot + Point<int16_t>(1, edge - 1));
						ColorBox(1, ICON_SIZE, Color::Name::SILVERCHALICE, 1.0f).draw(slot + Point<int16_t>(0, 1));
						ColorBox(1, ICON_SIZE, Color::Name::SILVERCHALICE, 1.0f).draw(slot + Point<int16_t>(edge - 1, 1));

						ItemData::get(rewards[index]).get_icon(false).draw(
								position + reward_pos + Point<int16_t>(0, ICON_SIZE));
					}
				}
					break;
				case Info::FOUND_IN:
					for (int16_t i = 0; i < MAP_LINES; i++)
					{
						size_t index = info_page * MAP_LINES + i;

						if (index >= map_lines.size())
							break;

						const MapLine &line = map_lines[index];
						int16_t indent = line.street ? 0 : MAP_INDENT;

						line.text.draw(position + MAPS + Point<int16_t>(indent, MAP_LINE * i));
					}

					break;
			}
		}

		UIElement::draw_buttons(alpha);
	}

	void UIMonsterBook::update()
	{
		UIElement::update();

		searchfield.update(position);
		mob_stand.update();
	}

	void UIMonsterBook::toggle_active()
	{
		UIElement::toggle_active();

		close_menu();
		UI::get().clear_tooltip(Tooltip::Parent::MONSTERBOOK);
	}

	bool UIMonsterBook::is_in_range(Point<int16_t> cursorpos) const
	{
		Point<int16_t> lt = position - Point<int16_t>(LEFT_REACH, 0);
		Point<int16_t> rb = position + Point<int16_t>(RIGHT_REACH, dimension.y());

		if (menu_card)
		{
			Point<int16_t> menu_rb = position + menu_position + Point<int16_t>(MENU_WIDTH, MENU_ROW * 2);

			if (Rectangle<int16_t>(position + menu_position, menu_rb).contains(cursorpos))
				return true;
		}

		return Rectangle<int16_t>(lt, rb).contains(cursorpos);
	}

	void UIMonsterBook::remove_cursor()
	{
		UIDragElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::MONSTERBOOK);
	}

	Cursor::State UIMonsterBook::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (tab != SUMMARY)
		{
			if (Cursor::State new_state = searchfield.send_cursor(cursorpos, clicked))
				return new_state;
		}

		// Pressing a menu button closes the menu, so remember it was open.
		bool menu_open = menu_card != 0;
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			return dstate;

		Point<int16_t> cursor_relative = cursorpos - position;

		// A click that closed the cover menu ends there, even while the
		// button is still held over a card.
		if (menu_click)
		{
			if (clicked)
				return dstate;

			menu_click = false;
		}

		// The cover menu's buttons take the click; anywhere else closes it.
		if (menu_open)
		{
			Rectangle<int16_t> menu = Rectangle<int16_t>(menu_position,
														 menu_position + Point<int16_t>(MENU_WIDTH, MENU_ROW * 2));

			if (clicked)
				menu_click = true;

			if (menu.contains(cursor_relative))
				return dstate;

			if (clicked)
			{
				close_menu();

				return dstate;
			}
		}

		if (int32_t reward = reward_by_position(cursor_relative))
		{
			UI::get().show_item(Tooltip::Parent::MONSTERBOOK, reward);

			return Cursor::State::IDLE;
		}

		UI::get().clear_tooltip(Tooltip::Parent::MONSTERBOOK);

		if (int32_t card = card_by_position(cursor_relative))
		{
			if (clicked)
			{
				select(card);

				return Cursor::State::CLICKING;
			}

			return Cursor::State::CANCLICK;
		}

		return dstate;
	}

	void UIMonsterBook::rightclick(Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor_relative = cursorpos - position;
		int32_t card = card_by_position(cursor_relative);

		if (card && book.get_level(card) > 0)
		{
			select(card);
			open_menu(card, cursor_relative);
		}
	}

	void UIMonsterBook::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			if (menu_card)
				close_menu();
			else
				close();
		}
	}

	UIElement::Type UIMonsterBook::get_type() const
	{
		return TYPE;
	}

	void UIMonsterBook::update_cards()
	{
		master_level.change_text(std::to_string(book.get_book_level()));

		int16_t special = book.count_cards(true);
		int16_t basic = book.count_cards(false);

		total_cards.change_text(std::to_string(special + basic));
		special_cards.change_text(std::to_string(special));
		basic_cards.change_text(std::to_string(basic));

		// Collecting more copies may show more about the selected monster.
		if (selected)
		{
			uint16_t old_info = info;
			int16_t old_page = info_page;

			select(selected);

			if (info_unlocked(old_info))
			{
				change_info(old_info);
				change_info_page(old_page);
			}
		}
	}

	Button::State UIMonsterBook::button_pressed(uint16_t id)
	{
		switch (id)
		{
			case Buttons::BT_CLOSE:
				close();
				break;
			case Buttons::BT_SEARCH:
				search(searchfield.get_text());
				break;
			case Buttons::BT_PREV:
				change_page(page - 1);
				break;
			case Buttons::BT_NEXT:
				change_page(page + 1);
				break;
			case Buttons::BT_INFO_PREV:
				change_info_page(info_page - 1);
				break;
			case Buttons::BT_INFO_NEXT:
				change_info_page(info_page + 1);
				break;
			case Buttons::BT_REGISTER:
				MonsterBookCoverPacket(menu_card).dispatch();
				close_menu();
				break;
			case Buttons::BT_RELEASE:
				MonsterBookCoverPacket(0).dispatch();
				close_menu();
				break;
			case Buttons::BT_INFO:
				change_tab(SUMMARY);
				break;
			default:
				if (id >= Buttons::BT_TAB0 && id < Buttons::BT_TAB0 + TABS)
					change_tab(id - Buttons::BT_TAB0);
				else if (id >= Buttons::BT_RIGHTTAB0 && id < Buttons::BT_RIGHTTAB0 + Info::NUM_INFO)
					change_info(id - Buttons::BT_RIGHTTAB0);

				break;
		}

		update_buttons();

		return buttons[id]->is_active() && buttons[id]->get_state() == Button::State::DISABLED
			   ? Button::State::DISABLED : Button::State::NORMAL;
	}

	void UIMonsterBook::change_tab(uint16_t new_tab)
	{
		close_menu();

		tab = new_tab;
		page = 0;
		selected = 0;

		searchfield.set_state(tab == SUMMARY ? Textfield::State::DISABLED : Textfield::State::NORMAL);

		// A category opens on its first card.
		if (tab != SUMMARY && !catalogue()[tab].empty())
			select(catalogue()[tab].front().id);

		update_buttons();
	}

	void UIMonsterBook::change_page(int16_t new_page)
	{
		if (new_page < 0 || new_page >= page_count())
			return;

		page = new_page;

		update_buttons();
	}

	void UIMonsterBook::select(int32_t card)
	{
		const Card *found = find_card(card);

		if (!found)
			return;

		selected = card;

		int32_t mob = found->mob;
		int8_t level = book.get_level(card);

		mob_name.change_text(mob_name_of(mob));

		std::string strid = string_format::extend_id(mob, 7);
		nl::node src = nl::nx::mob[strid + ".img"];
		nl::node info_node = src["info"];

		// Some monsters borrow another's sprites.
		if (info_node["link"])
			src = nl::nx::mob[string_format::extend_id(info_node["link"], 7) + ".img"];

		mob_stand = src["fly"].size() > 0 ? Animation(src["fly"]) : Animation(src["stand"]);

		// The more copies, the more the book tells: HP and MP from the
		// second, then the episode, the drops, and where it lives.
		bool known = level >= 2;
		mob_hp.change_text("HP : " + (known ? std::to_string(info_node["maxHP"].get_integer()) : "???"));
		mob_mp.change_text("MP : " + (known ? std::to_string(info_node["maxMP"].get_integer()) : "???"));

		nl::node entry = nl::nx::string["MonsterBook.img"][std::to_string(mob)];

		episode_pages = paginate(entry["episode"]);

		rewards.clear();

		for (int32_t item : numbered(entry["reward"]))
			if (ItemData::get(item).is_valid())
				rewards.push_back(item);

		// Maps are listed under their street, as in the original.
		map_lines.clear();

		std::string street;

		for (int32_t map : numbered(entry["map"]))
		{
			NxHelper::Map::MapInfo map_info = NxHelper::Map::get_map_info_by_id(map);

			if (map_info.name.empty())
				continue;

			if (map_info.street_name != street)
			{
				street = map_info.street_name;
				map_lines.push_back({ Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK, street), true });
			}

			map_lines.push_back({ Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, map_info.name), false });
		}

		change_info(Info::BASIC);
	}

	void UIMonsterBook::change_info(uint16_t new_info)
	{
		if (!selected || !info_unlocked(new_info))
			return;

		info = new_info;
		info_page = 0;

		update_buttons();
	}

	void UIMonsterBook::change_info_page(int16_t new_page)
	{
		if (new_page < 0 || new_page >= info_page_count())
			return;

		info_page = new_page;

		update_buttons();
	}

	void UIMonsterBook::update_buttons()
	{
		bool cards = tab != SUMMARY;

		buttons[Buttons::BT_SEARCH]->set_active(cards);
		buttons[Buttons::BT_PREV]->set_active(cards);
		buttons[Buttons::BT_NEXT]->set_active(cards);
		buttons[Buttons::BT_PREV]->set_state(page > 0 ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_NEXT]->set_state(page + 1 < page_count() ? Button::State::NORMAL : Button::State::DISABLED);

		bool paged = selected && info != Info::BASIC;

		buttons[Buttons::BT_INFO_PREV]->set_active(paged);
		buttons[Buttons::BT_INFO_NEXT]->set_active(paged);
		buttons[Buttons::BT_INFO_PREV]->set_state(info_page > 0 ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_INFO_NEXT]->set_state(
				info_page + 1 < info_page_count() ? Button::State::NORMAL : Button::State::DISABLED);

		for (uint16_t i = 0; i < TABS; i++)
			buttons[Buttons::BT_TAB0 + i]->set_active(i != tab);

		buttons[Buttons::BT_INFO]->set_active(tab != SUMMARY);

		for (uint16_t i = 0; i < Info::NUM_INFO; i++)
			buttons[Buttons::BT_RIGHTTAB0 + i]->set_active(selected && i != info && info_unlocked(i));
	}

	void UIMonsterBook::search(const std::string &text)
	{
		std::string wanted = lowercase(text);

		if (wanted.empty())
			return;

		for (uint16_t t = 0; t < TABS; t++)
		{
			const std::vector<Card> &cards = catalogue()[t];

			for (size_t i = 0; i < cards.size(); i++)
			{
				if (lowercase(mob_name_of(cards[i].mob)).find(wanted) == std::string::npos)
					continue;

				std::string kept = searchfield.get_text();

				change_tab(t);
				change_page(i / PER_PAGE);
				select(cards[i].id);
				searchfield.change_text(kept);

				return;
			}
		}
	}

	void UIMonsterBook::open_menu(int32_t card, Point<int16_t> at)
	{
		menu_card = card;
		menu_position = at;

		bool on_cover = card == book.get_cover();

		buttons[Buttons::BT_REGISTER]->set_position(at);
		buttons[Buttons::BT_RELEASE]->set_position(at + Point<int16_t>(0, MENU_ROW));
		buttons[Buttons::BT_REGISTER]->set_active(true);
		buttons[Buttons::BT_RELEASE]->set_active(true);
		buttons[Buttons::BT_REGISTER]->set_state(on_cover ? Button::State::DISABLED : Button::State::NORMAL);
		buttons[Buttons::BT_RELEASE]->set_state(on_cover ? Button::State::NORMAL : Button::State::DISABLED);
	}

	void UIMonsterBook::close_menu()
	{
		menu_card = 0;

		buttons[Buttons::BT_REGISTER]->set_active(false);
		buttons[Buttons::BT_RELEASE]->set_active(false);
	}

	int32_t UIMonsterBook::card_by_position(Point<int16_t> cursorpos) const
	{
		if (tab == SUMMARY)
			return 0;

		Point<int16_t> relative = cursorpos - CARD;

		if (relative.x() < 0 || relative.y() < 0)
			return 0;

		int16_t column = relative.x() / CARD_PITCH.x();
		int16_t row = relative.y() / CARD_PITCH.y();

		if (column >= COLUMNS || row >= ROWS)
			return 0;

		if (relative.x() % CARD_PITCH.x() >= CARD_SIZE.x() || relative.y() % CARD_PITCH.y() >= CARD_SIZE.y())
			return 0;

		size_t index = page * PER_PAGE + row * COLUMNS + column;
		const std::vector<Card> &cards = catalogue()[tab];

		return index < cards.size() ? cards[index].id : 0;
	}

	int32_t UIMonsterBook::reward_by_position(Point<int16_t> cursorpos) const
	{
		if (!selected || info != Info::DROPPING)
			return 0;

		Point<int16_t> relative = cursorpos - REWARD;

		if (relative.x() < 0 || relative.y() < 0)
			return 0;

		int16_t column = relative.x() / REWARD_PITCH.x();
		int16_t row = relative.y() / REWARD_PITCH.y();

		if (column >= REWARD_COLUMNS || row >= REWARD_ROWS)
			return 0;

		if (relative.x() % REWARD_PITCH.x() >= ICON_SIZE || relative.y() % REWARD_PITCH.y() >= ICON_SIZE)
			return 0;

		size_t index = info_page * REWARD_COLUMNS * REWARD_ROWS + row * REWARD_COLUMNS + column;

		return index < rewards.size() ? rewards[index] : 0;
	}

	const UIMonsterBook::Card *UIMonsterBook::find_card(int32_t card) const
	{
		size_t category = card / 1000 % 10;

		if (category >= TABS)
			return nullptr;

		for (const Card &entry : catalogue()[category])
			if (entry.id == card)
				return &entry;

		return nullptr;
	}

	int16_t UIMonsterBook::page_count() const
	{
		if (tab == SUMMARY)
			return 1;

		size_t cards = catalogue()[tab].size();

		return std::max<int16_t>(1, (cards + PER_PAGE - 1) / PER_PAGE);
	}

	int16_t UIMonsterBook::info_page_count() const
	{
		switch (info)
		{
			case Info::EPISODE:
				return std::max<int16_t>(1, episode_pages.size());
			case Info::DROPPING:
			{
				int16_t per_page = REWARD_COLUMNS * REWARD_ROWS;

				return std::max<int16_t>(1, (rewards.size() + per_page - 1) / per_page);
			}
			case Info::FOUND_IN:
				return std::max<int16_t>(1, (map_lines.size() + MAP_LINES - 1) / MAP_LINES);
			default:
				return 1;
		}
	}

	bool UIMonsterBook::info_unlocked(uint16_t page_info) const
	{
		int8_t level = book.get_level(selected);

		switch (page_info)
		{
			case Info::BASIC:
				return true;
			case Info::EPISODE:
				return level >= 3;
			case Info::DROPPING:
				return level >= 4;
			case Info::FOUND_IN:
				return level >= 5;
			default:
				return false;
		}
	}

	void UIMonsterBook::close()
	{
		close_menu();
		UI::get().clear_tooltip(Tooltip::Parent::MONSTERBOOK);
		deactivate();
	}
}
