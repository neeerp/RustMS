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
#pragma once

#include <cstdint>

#include "../UIDragElement.h"

#include "../Components/Charset.h"
#include "../Components/Textfield.h"

#include "../../Character/MonsterBook.h"
#include "../../Graphics/Animation.h"

namespace ms
{
	// The v83 Monster Book (UIClassic.nx UIWindow.img/MonsterBook), laid out
	// from period footage: the book's summary behind the "!" tab, a tab of
	// cards for each category on the left page, and the chosen card's
	// monster on the right page.
	class UIMonsterBook : public UIDragElement<PosMONSTERBOOK>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MONSTERBOOK;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIMonsterBook(const MonsterBook &book);

		void draw(float alpha) const override;

		void update() override;

		void toggle_active() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void rightclick(Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Redraw after a card was added or the cover changed.
		void update_cards();

	protected:
		Button::State button_pressed(uint16_t id) override;

	private:
		struct Card
		{
			int32_t id;
			int32_t mob;
		};

		// A line of the Found In page: a street, or a map on it.
		struct MapLine
		{
			Text text;
			bool street;
		};

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_SEARCH,
			BT_PREV,
			BT_NEXT,
			BT_INFO_PREV,
			BT_INFO_NEXT,
			BT_REGISTER,
			BT_RELEASE,
			BT_INFO,
			BT_TAB0,
			BT_RIGHTTAB0 = BT_TAB0 + 9
		};

		// The pages about a monster on the right.
		enum Info : uint16_t
		{
			BASIC,
			EPISODE,
			DROPPING,
			FOUND_IN,
			NUM_INFO
		};

		// The card tabs, one per category; the "!" tab is the summary.
		static constexpr uint16_t TABS = 9;
		static constexpr uint16_t SUMMARY = TABS;
		static constexpr int16_t COLUMNS = 5;
		static constexpr int16_t ROWS = 5;
		static constexpr int16_t PER_PAGE = COLUMNS * ROWS;

		// Every card in Item.nx, by category and then id.
		static const std::vector<std::vector<Card>> &catalogue();

		void change_tab(uint16_t new_tab);

		void change_page(int16_t new_page);

		void select(int32_t card);

		void change_info(uint16_t new_info);

		void change_info_page(int16_t new_page);

		void update_buttons();

		void search(const std::string &text);

		void open_menu(int32_t card, Point<int16_t> position);

		void close_menu();

		// The card under a position relative to the window, or 0.
		int32_t card_by_position(Point<int16_t> cursorpos) const;

		// The reward shown under a position relative to the window, or 0.
		int32_t reward_by_position(Point<int16_t> cursorpos) const;

		const Card *find_card(int32_t card) const;

		int16_t page_count() const;

		int16_t info_page_count() const;

		bool info_unlocked(uint16_t info) const;

		void close();

		const MonsterBook &book;

		Texture cardslot;
		Texture infopage;
		Texture select_frame;
		Texture cover_frame;
		Texture fullmark;
		std::vector<Texture> book_icons;
		std::vector<Texture> lefttabs[2];
		Texture infotab[2];
		std::vector<Texture> righttabs[3];
		Charset countset;
		Textfield searchfield;

		Text master_level;
		Text total_cards;
		Text special_cards;
		Text basic_cards;

		uint16_t tab;
		int16_t page;
		int32_t selected;
		uint16_t info;
		int16_t info_page;

		// The selected card's monster.
		Text mob_name;
		Animation mob_stand;
		Text mob_hp;
		Text mob_mp;
		std::vector<Text> episode_pages;
		std::vector<int32_t> rewards;
		std::vector<MapLine> map_lines;

		int32_t menu_card;
		Point<int16_t> menu_position;
		bool menu_click;
	};
}
