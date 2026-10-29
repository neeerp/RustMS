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
#include "UICharInfo.h"

#include "../Components/MapleButton.h"

#include "../../Character/CharInfoLists.h"
#include "../../Character/Job.h"
#include "../../Character/MedalTagLayout.h"
#include "../../Character/MountRules.h"
#include "../../Character/Look/TamingMob.h"
#include "../../Data/EquipData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/FamilyActions.h"
#include "../../IO/PartyCommands.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UITradingRoom.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <map>
#include <optional>

namespace ms
{
	namespace
	{
		// Laid out from footage and the v83 client, against the window's
		// top-left corner.
		constexpr int16_t WIDTH = 275;
		// Each mode's window height, and the row its mode buttons sit on.
		constexpr int16_t HEIGHTS[] = { 199, 400, 362, 404 };
		constexpr int16_t BOTTOMS[] = { 177, 378, 340, 382 };
		constexpr int16_t TAMING_X = 5, PET_X = 93, COLLECTION_X = 145, EXCEPTION_X = 199;

		const Point<int16_t> CLOSE = { 256, 6 };
		const Point<int16_t> FAMILY_AT = { 105, 23 };
		const Point<int16_t> PARTY = { 149, 23 };
		const Point<int16_t> TRADE = { 149, 42 };
		const Point<int16_t> ITEM = { 208, 22 };
		const Point<int16_t> WISH = { 208, 41 };
		const Point<int16_t> FAME_UP = { 237, 114 };
		const Point<int16_t> FAME_DOWN = { 252, 114 };

		// The character stands on the shadow in the portrait box, with
		// their name in the box below.
		const Point<int16_t> PORTRAIT = { 52, 133 };
		const Point<int16_t> NAME = { 53, 139 };

		// The taming box below has the rider on their mount over a shadow,
		// the mount's name under them, and its values in two columns.
		const Point<int16_t> RIDER = { 50, 264 };
		const Point<int16_t> MOUNT_NAME = { 53, 264 };
		const Point<int16_t> MOUNT_TYPE = { 58, 287 };
		const Point<int16_t> MOUNT_LEVEL = { 58, 305 };
		const Point<int16_t> MOUNT_EXP = { 189, 287 };
		const Point<int16_t> MOUNT_TIREDNESS = { 189, 305 };

		// The pet box (FUN_00901185, FUN_0090433a): the pet stands on the
		// shadow over its name, the first wearing the boss crown, with its
		// values in two columns.
		const Point<int16_t> PET_STAND = { 53, 299 };
		const Point<int16_t> CROWN = { 44, 209 };
		const Point<int16_t> PET_NAME = { 53, 301 };
		const Point<int16_t> PET_TYPE = { 58, 325 };
		const Point<int16_t> PET_LEVEL = { 58, 342 };
		const Point<int16_t> PET_CLOSENESS = { 189, 324 };
		const Point<int16_t> PET_FULLNESS = { 189, 342 };
		// A button per pet, placed by its top left (FUN_008ffe3b).
		const Point<int16_t> PET_BUTTON = { 168, 177 };
		constexpr int16_t PET_BUTTON_STEP = 34;
		constexpr size_t MAX_PETS = 3;

		// The pet's equipment, three rows at a time: the icon, the name
		// cut to 100 pixels, and the level it needs.
		constexpr int16_t ITEM_ROWS = 3, ITEM_STEP = 38;
		constexpr size_t ITEM_NAME_WIDTH = 100;
		const Point<int16_t> ITEM_ICON = { 106, 248 };
		const Point<int16_t> ITEM_NAME = { 147, 211 };
		const Point<int16_t> ITEM_REQ = { 147, 236 };
		const Point<int16_t> ITEM_REQ_LEVEL = { 183, 229 };
		// The scrollbar (id 0x7e1) is 112 tall from (252, 215); ours puts its
		// lower arrow's top at the range's end.
		constexpr int16_t ITEM_SLIDER_X = 252;
		constexpr Range<int16_t> ITEM_SLIDER = Range<int16_t>(215, 215 + 112 - 13);
		// The taming box draws the same rows higher up (0x902641): icons
		// and the ReqLv art 36 higher, the text 38.
		const Point<int16_t> TAMING_ICON = { 106, 212 };
		const Point<int16_t> TAMING_NAME = { 147, 173 };
		const Point<int16_t> TAMING_REQ = { 147, 200 };
		const Point<int16_t> TAMING_REQ_LEVEL = { 183, 191 };
		constexpr Range<int16_t> TAMING_SLIDER = Range<int16_t>(177, 177 + 112 - 13);

		// Values sit right of the labels; a row's text has its cap top 4
		// below the row's top.
		constexpr int16_t VALUE_X = 160;
		constexpr int16_t ROW_LEVEL = 76, ROW_STEP = 18;
		// From where a line of text is drawn to its cap top.
		constexpr int16_t TEXT_Y = -9;

		// The item and wish lists open beside the window, over its edge.
		const Point<int16_t> PANEL = { 270, 0 };
		const Point<int16_t> PANEL_HIDE = { 220, 6 };
		const Point<int16_t> PANEL_SIZE = { 240, 162 };
		// The panels' rows (FUN_008fd4c0, FUN_008fde08), against the panel:
		// the icon, the name, and for equipment the ReqLv art and the level.
		constexpr int16_t PANEL_ROWS = 3, PANEL_STEP = 40;
		const Point<int16_t> PANEL_ICON = { 17, 64 };
		const Point<int16_t> PANEL_NAME = { 57, 26 };
		// A wish row's BtPresent, against the panel.
		const Point<int16_t> PANEL_PRESENT = { 165, 50 };
		const Point<int16_t> PANEL_REQ = { 57, 53 };
		const Point<int16_t> PANEL_REQ_LEVEL = { 93, 44 };
		// The scrollbar (id 0x3e8) is 112 tall from (218, 33).
		constexpr int16_t PANEL_SLIDER_X = 218;
		constexpr Range<int16_t> PANEL_SLIDER = Range<int16_t>(33, 33 + 112 - 13);

		// The collection page (0x902ae0): the book's level and its icon,
		// the cover's mob, the cards, the medal worn and the titles earned,
		// four at a time. Text goes 6 above v83's y, as for the boxes.
		const Point<int16_t> LEVEL_ICON = { 14, 182 };
		const Point<int16_t> BOOK_LEVEL = { 101, 175 };
		const Point<int16_t> COVER_NAME = { 101, 193 };
		const Point<int16_t> BOOK_TOTAL = { 101, 211 };
		const Point<int16_t> BOOK_NORMAL = { 168, 211 };
		const Point<int16_t> BOOK_SPECIAL = { 236, 211 };
		const Point<int16_t> MEDAL_ICON = { 8, 278 };
		const Point<int16_t> MEDAL_NAME = { 101, 241 };
		const Point<int16_t> MEDAL_COUNT = { 101, 259 };
		const Point<int16_t> MEDAL_ROW = { 52, 295 };
		constexpr int16_t MEDAL_ROWS = 4, MEDAL_STEP = 18;
		// The scrollbar (id 0x7e1) is 73 tall from (252, 298).
		constexpr int16_t MEDAL_SLIDER_X = 252;
		constexpr Range<int16_t> MEDAL_SLIDER = Range<int16_t>(298, 298 + 73 - 13);

		// A wish's item and count, from Etc.nx Commodity.img by its SN.
		std::optional<char_info::Wish> commodity(int32_t sn)
		{
			static const std::map<int32_t, char_info::Wish> all = []()
			{
				std::map<int32_t, char_info::Wish> found;

				for (nl::node item : nl::nx::etc["Commodity.img"])
				{
					auto id = static_cast<int32_t>(item["SN"].get_integer());
					found[id] = { id, static_cast<int32_t>(item["ItemId"].get_integer()),
								  static_cast<int16_t>(item["Count"].get_integer(1)) };
				}

				return found;
			}();

			auto found = all.find(sn);

			if (found == all.end())
				return std::nullopt;

			return found->second;
		}

		// The exception window opens under the window's right edge, lower
		// beside a box (FUN_009009bf).
		const Point<int16_t> EXCEPTIONS = { 270, 34 };
		constexpr int16_t EXCEPTIONS_PET_Y = 196;
	}

	UICharInfo::UICharInfo(int32_t id) : UIDragElement<PosCHARINFO>(Point<int16_t>(WIDTH, 20)), cid(id), known(false), mode(Mode::MAIN), panel(Panel::NO_PANEL)
	{
		nl::node UserInfo = NxFiles::ui_classic["UIWindow.img"]["UserInfo"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrounds[Mode::MAIN] = UserInfo["backgrnd"];
		backgrounds[Mode::PET] = UserInfo["backgrnd2"];
		backgrounds[Mode::TAMING] = UserInfo["backgrnd5"];
		backgrounds[Mode::COLLECTION] = UserInfo["backgrnd7"];
		item_list = UserInfo["backgrnd4"];
		wish_list = UserInfo["backgrnd3"];

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], CLOSE);
		buttons[Buttons::BT_FAMILY] = std::make_unique<MapleButton>(UserInfo["BtFamily"], FAMILY_AT);
		buttons[Buttons::BT_PARTY] = std::make_unique<MapleButton>(UserInfo["BtParty"], PARTY);
		buttons[Buttons::BT_TRADE] = std::make_unique<MapleButton>(UserInfo["BtTrade"], TRADE);
		buttons[Buttons::BT_ITEM] = std::make_unique<MapleButton>(UserInfo["BtItem"], ITEM);
		buttons[Buttons::BT_WISH] = std::make_unique<MapleButton>(UserInfo["BtWish"], WISH);
		buttons[Buttons::BT_FAME_UP] = std::make_unique<MapleButton>(Basic["BtUP"], FAME_UP);
		buttons[Buttons::BT_FAME_DOWN] = std::make_unique<MapleButton>(Basic["BtDown"], FAME_DOWN);
		buttons[Buttons::BT_PET_SHOW] = std::make_unique<MapleButton>(UserInfo["BtPetShow"]);
		buttons[Buttons::BT_PET_HIDE] = std::make_unique<MapleButton>(UserInfo["BtPetHide"]);
		buttons[Buttons::BT_TAMING_SHOW] = std::make_unique<MapleButton>(UserInfo["BtTamingShow"]);
		buttons[Buttons::BT_TAMING_HIDE] = std::make_unique<MapleButton>(UserInfo["BtTamingHide"]);
		buttons[Buttons::BT_COLLECTION_SHOW] = std::make_unique<MapleButton>(UserInfo["BtCollectionShow"]);
		buttons[Buttons::BT_COLLECTION_HIDE] = std::make_unique<MapleButton>(UserInfo["BtCollectionHide"]);
		buttons[Buttons::BT_EXCEPTION] = std::make_unique<MapleButton>(UserInfo["BtExceptionShow"]);
		buttons[Buttons::BT_PANEL_HIDE] = std::make_unique<MapleButton>(Basic["BtHide"], PANEL + PANEL_HIDE);

		for (int16_t i = 0; i < PANEL_ROWS; i++)
			buttons[Buttons::BT_PRESENT1 + i] = std::make_unique<MapleButton>(UserInfo["BtPresent"], PANEL + PANEL_PRESENT + Point<int16_t>(0, PANEL_STEP * i));

		for (size_t i = 0; i < MAX_PETS; i++)
		{
			nl::node src = UserInfo["BtPet" + std::to_string(i + 1)];
			Point<int16_t> origin = Texture(src["normal"]["0"]).get_origin();
			Point<int16_t> pos = PET_BUTTON + Point<int16_t>(PET_BUTTON_STEP * static_cast<int16_t>(i), 0) + origin;

			buttons[Buttons::BT_PET1 + i] = std::make_unique<MapleButton>(src, pos);
		}

		crown = UserInfo["bossPetCrown"];
		req_level = UserInfo["ReqLv"];

		item_slider = Slider(Slider::Type::CLASSIC, ITEM_SLIDER, ITEM_SLIDER_X, ITEM_ROWS, 0, [&](bool upwards)
		{
			item_offset += upwards ? -1 : 1;
		});
		taming_slider = Slider(Slider::Type::CLASSIC, TAMING_SLIDER, ITEM_SLIDER_X, ITEM_ROWS, 0, [](bool) {});
		panel_slider = Slider(Slider::Type::CLASSIC, PANEL_SLIDER, PANEL_SLIDER_X, PANEL_ROWS, 0, [&](bool upwards)
		{
			panel_offset += upwards ? -1 : 1;
			show_presents();
		});
		medal_slider = Slider(Slider::Type::CLASSIC, MEDAL_SLIDER, MEDAL_SLIDER_X, MEDAL_ROWS, 0, [&](bool upwards)
		{
			medal_offset += upwards ? -1 : 1;
			show_medals();
		});

		nl::node icons = NxFiles::ui_classic["UIWindow.img"]["MonsterBook"]["icon"];

		for (size_t i = 0; i < icons.size(); i++)
			level_icons.emplace_back(icons[std::to_string(i)]);

		for (Text *text : { &book_level, &cover_name, &book_total, &book_normal, &book_special, &medal_name, &medal_count })
			*text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		// The pet and taming boxes open once CHAR_INFO says there are pets
		// or a mount. BtFamily is always on (report b045).
		for (auto button : { Buttons::BT_EXCEPTION, Buttons::BT_PET_SHOW, Buttons::BT_TAMING_SHOW })
			buttons[button]->set_state(Button::State::DISABLED);

		// v83 offers fame only from level 15 (FUN_009007a0), and never for
		// oneself (FUN_00903b86).
		auto &player = Stage::get().get_player();

		if (cid == player.get_oid() || player.get_stats().get_stat(MapleStat::Id::LEVEL) < 15)
			for (auto button : { Buttons::BT_FAME_UP, Buttons::BT_FAME_DOWN })
				buttons[button]->set_state(Button::State::DISABLED);

		// Nor does it invite or trade with oneself (FUN_00903b86).
		if (cid == player.get_oid())
			for (auto button : { Buttons::BT_PARTY, Buttons::BT_TRADE })
				buttons[button]->set_state(Button::State::DISABLED);

		name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);
		level = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		// v83 draws the job in Tahoma 9 (font 0x2c), smaller than the other rows, so that its long names, such as
		// "Arch Mage(Ice,Lightning)", fit; 10 px Arimo is the nearest we have.
		job = Text(Text::Font::A10M, Text::Alignment::LEFT, Color::Name::BLACK);
		fame = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		guild = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		alliance = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		mount_name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		for (Text *text : { &mount_type, &mount_level, &mount_exp, &mount_tiredness })
			*text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		// Copy what is shown: the character may leave the map while the
		// window is open. The portrait stands even if they ride.
		if (auto character = Stage::get().get_character(cid))
		{
			look = character->get_look();
			look.set_vehicle(0);
			name.change_text(character->get_name());
			known = true;
		}

		riding = false;

		list_worn();
		set_mode(Mode::MAIN);
		set_panel(Panel::NO_PANEL);
	}

	void UICharInfo::draw(float alpha) const
	{
		if (exceptions)
			exceptions->draw(alpha);

		backgrounds[mode].draw(position);

		if (panel == Panel::ITEMS)
			item_list.draw(position + PANEL);
		else if (panel == Panel::WISHES)
			wish_list.draw(position + PANEL);

		draw_panel();

		if (mode == Mode::COLLECTION)
			draw_collection();

		if (known)
			look.draw(position + PORTRAIT, false, Stance::Id::STAND1, Expression::Id::DEFAULT);

		name.draw(position + NAME);

		const Text *rows[] = { &level, &job, &fame, &guild, &alliance };

		for (size_t i = 0; i < 5; i++)
			rows[i]->draw(position + Point<int16_t>(VALUE_X, ROW_LEVEL + ROW_STEP * i + 4 + TEXT_Y));

		if (mode == Mode::TAMING)
		{
			if (riding)
				rider.draw(rider_args + position, Stance::Id::STAND1, Expression::Id::DEFAULT);

			mount_name.draw(position + MOUNT_NAME);
			mount_type.draw(position + MOUNT_TYPE);
			mount_level.draw(position + MOUNT_LEVEL);
			mount_exp.draw(position + MOUNT_EXP);
			mount_tiredness.draw(position + MOUNT_TIREDNESS);

			draw_items(taming_rows, 0, true);
			taming_slider.draw(position);
		}

		if (mode == Mode::PET)
		{
			if (selected < pet_pages.size())
			{
				const PetPage &pet = pet_pages[selected];
				pet.name.draw(position + PET_NAME);
				pet.type.draw(position + PET_TYPE);
				pet.level.draw(position + PET_LEVEL);
				pet.closeness.draw(position + PET_CLOSENESS);
				pet.fullness.draw(position + PET_FULLNESS);
				pet.look.draw_standing(position + PET_STAND, alpha);

				if (selected == 0)
					crown.draw(position + CROWN);
			}

			draw_items(item_rows, item_offset, false);
			item_slider.draw(position);
		}

		UIElement::draw_buttons(alpha);
	}

	void UICharInfo::draw_items(const std::vector<ItemRow> &rows, int16_t offset, bool taming) const
	{
		for (int16_t r = 0; r < ITEM_ROWS; r++)
		{
			size_t i = static_cast<size_t>(offset + r);

			if (offset + r < 0 || i >= rows.size())
				break;

			const ItemRow &row = rows[i];
			Point<int16_t> step = Point<int16_t>(0, ITEM_STEP * r);

			row.icon.draw(position + (taming ? TAMING_ICON : ITEM_ICON) + step);
			row.name.draw(position + (taming ? TAMING_NAME : ITEM_NAME) + step);
			req_level.draw(position + (taming ? TAMING_REQ : ITEM_REQ) + step);

			if (row.equip)
				row.level.draw(position + (taming ? TAMING_REQ_LEVEL : ITEM_REQ_LEVEL) + step);
		}
	}

	void UICharInfo::draw_panel() const
	{
		if (panel == Panel::NO_PANEL)
			return;

		const std::vector<ItemRow> &rows = panel_rows();
		Point<int16_t> at = position + PANEL;

		for (int16_t r = 0; r < PANEL_ROWS; r++)
		{
			size_t i = static_cast<size_t>(panel_offset + r);

			if (panel_offset + r < 0 || i >= rows.size())
				break;

			const ItemRow &row = rows[i];
			Point<int16_t> step = Point<int16_t>(0, PANEL_STEP * r);

			row.icon.draw(at + PANEL_ICON + step);
			row.name.draw(at + PANEL_NAME + step);

			if (row.equip)
			{
				req_level.draw(at + PANEL_REQ + step);
				row.level.draw(at + PANEL_REQ_LEVEL + step);
			}
		}

		panel_slider.draw(at);
	}

	void UICharInfo::draw_collection() const
	{
		level_icon.draw(position + LEVEL_ICON);
		book_level.draw(position + BOOK_LEVEL);
		cover_name.draw(position + COVER_NAME);
		book_total.draw(position + BOOK_TOTAL);
		book_normal.draw(position + BOOK_NORMAL);
		book_special.draw(position + BOOK_SPECIAL);
		medal_icon.draw(position + MEDAL_ICON);
		medal_name.draw(position + MEDAL_NAME);
		medal_count.draw(position + MEDAL_COUNT);

		for (size_t i = 0; i < medal_titles.size(); i++)
			medal_titles[i].draw(position + MEDAL_ROW + Point<int16_t>(0, static_cast<int16_t>(MEDAL_STEP * i)));

		medal_slider.draw(position);
	}

	void UICharInfo::update()
	{
		UIDragElement::update();

		if (mode == Mode::PET && selected < pet_pages.size())
			pet_pages[selected].look.update_standing();

		if (exceptions)
		{
			exceptions->set_position(exceptions_position());
			exceptions->update();
		}

		Button &exception = *buttons[Buttons::BT_EXCEPTION];
		bool shows = can_show_exceptions();

		if (shows && exception.get_state() == Button::State::DISABLED)
			exception.set_state(Button::State::NORMAL);
		else if (!shows)
			exception.set_state(Button::State::DISABLED);
	}

	Button::State UICharInfo::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_CLOSE:
			close();
			break;
		case Buttons::BT_EXCEPTION:
			toggle_exceptions();
			break;
		case Buttons::BT_ITEM:
			set_panel(panel == Panel::ITEMS ? Panel::NO_PANEL : Panel::ITEMS);
			break;
		case Buttons::BT_WISH:
			set_panel(panel == Panel::WISHES ? Panel::NO_PANEL : Panel::WISHES);
			break;
		case Buttons::BT_PANEL_HIDE:
			set_panel(Panel::NO_PANEL);
			break;
		case Buttons::BT_PRESENT1:
		case Buttons::BT_PRESENT2:
		case Buttons::BT_PRESENT3:
		{
			// Into the Cash Shop, which offers the wish as a gift to this character once it opens (0x8fe30d).
			size_t i = static_cast<size_t>(panel_offset + buttonid - Buttons::BT_PRESENT1);

			if (i < wishes.size())
				Stage::get().enter_cash_shop(cash_catalog::Present{ wishes[i].sn, name.get_text() });

			return Button::State::NORMAL;
		}
		case Buttons::BT_FAMILY:
			// The player's own opens the Family window, another's their tree.
			if (name.get_text() == Stage::get().get_player().get_stats().get_name())
				FamilyActions::toggle_window();
			else
				FamilyActions::open_tree(name.get_text());
			break;
		case Buttons::BT_FAME_UP:
			give_fame(true);
			break;
		case Buttons::BT_FAME_DOWN:
			give_fame(false);
			break;
		case Buttons::BT_TRADE:
			UITradingRoom::request(cid);
			break;
		case Buttons::BT_PARTY:
			invite();
			break;
		case Buttons::BT_PET_SHOW:
			set_mode(Mode::PET);
			break;
		case Buttons::BT_TAMING_SHOW:
			set_mode(Mode::TAMING);
			break;
		case Buttons::BT_COLLECTION_SHOW:
			set_mode(Mode::COLLECTION);
			break;
		case Buttons::BT_PET_HIDE:
			close_exceptions();
			set_mode(Mode::MAIN);
			break;
		case Buttons::BT_TAMING_HIDE:
		case Buttons::BT_COLLECTION_HIDE:
			set_mode(Mode::MAIN);
			break;
		case Buttons::BT_PET1:
		case Buttons::BT_PET2:
		case Buttons::BT_PET3:
			// The exception window stays only for the pet it shows.
			if (exceptions && exceptions->get_pet() != buttonid - Buttons::BT_PET1)
				close_exceptions();

			select_pet(buttonid - Buttons::BT_PET1);
			set_mode(Mode::PET);
			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	bool UICharInfo::is_in_range(Point<int16_t> cursorpos) const
	{
		if (Rectangle<int16_t>(position, position + dimension).contains(cursorpos) || on_exceptions(cursorpos))
			return true;

		Point<int16_t> panelpos = position + PANEL;

		return panel != Panel::NO_PANEL && Rectangle<int16_t>(panelpos, panelpos + PANEL_SIZE).contains(cursorpos);
	}

	Cursor::State UICharInfo::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		cursor = cursorpos;

		if (exceptions && !dragged)
		{
			if (exceptions->is_holding_button() || on_exceptions(cursorpos))
			{
				Cursor::State state = exceptions->send_cursor(clicked, cursorpos);

				if (!exceptions->is_active())
					toggle_exceptions();

				return state;
			}

			exceptions->remove_cursor();
		}

		if (on_panel(cursorpos))
		{
			Cursor::State state = panel_slider.send_cursor(cursorpos - position - PANEL, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}
		else
		{
			panel_slider.remove_cursor();
		}

		if (mode == Mode::PET || mode == Mode::COLLECTION)
		{
			Slider &slider = mode == Mode::PET ? item_slider : medal_slider;
			Cursor::State state = slider.send_cursor(cursorpos - position, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UICharInfo::remove_cursor()
	{
		UIDragElement::remove_cursor();

		item_slider.remove_cursor();
		panel_slider.remove_cursor();
		medal_slider.remove_cursor();

		if (exceptions)
			exceptions->remove_cursor();
	}

	void UICharInfo::send_scroll(double yoffset)
	{
		if (on_exceptions(cursor))
			exceptions->send_scroll(yoffset);
		else if (on_panel(cursor))
			panel_slider.send_scroll(yoffset);
		else if (mode == Mode::PET)
			item_slider.send_scroll(yoffset);
		else if (mode == Mode::COLLECTION)
			medal_slider.send_scroll(yoffset);
	}

	void UICharInfo::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UICharInfo::get_type() const
	{
		return TYPE;
	}

	void UICharInfo::update_stats(int32_t character_id, int16_t job_id, int8_t lv, int16_t f, std::string g, std::string a)
	{
		if (character_id != cid)
			return;

		level.change_text(std::to_string(static_cast<uint8_t>(lv)));
		job.change_text(Job(job_id).get_name());
		fame.change_text(std::to_string(f));
		guild.change_text(g.empty() ? "-" : g);
		alliance.change_text(a.empty() ? "-" : a);
	}

	void UICharInfo::update_mount(int32_t character_id, const Mount &mount)
	{
		if (character_id != cid || !shows_mount())
			return;

		int32_t mountid = look.get_equips().get_equip(EquipSlot::Id::TAMEDMOB);
		int32_t saddleid = look.get_equips().get_equip(EquipSlot::Id::SADDLE);

		if (mountid > 0)
		{
			std::string itemname = ItemData::get(mountid).get_name();
			mount_name.change_text(itemname);
			mount_type.change_text(itemname);
		}

		mount_level.change_text(std::to_string(mount.level));
		mount_exp.change_text(std::to_string(mount.exp));
		mount_tiredness.change_text(std::to_string(mount.tiredness));

		// The saddle and whatever is worn at -20, not the mount itself; HC
		// keeps no -20 part, and no v83 item seems to use it.
		taming_rows.clear();

		if (saddleid > 0)
			taming_rows.push_back(item_row(saddleid));

		taming_slider.setrows(0, ITEM_ROWS, static_cast<int16_t>(taming_rows.size()));

		riding = known && mountid > 0 && saddleid > 0;

		if (riding)
		{
			rider = look;
			rider.set_vehicle(mountid);

			nl::node info = nl::nx::character["TamingMob"][string_format::extend_id(mountid, 8) + ".img"]["info"];
			mount::InfoPortrait shown = mount::fixed_info_portrait(mountid).value_or(mount::InfoPortrait{
				static_cast<int32_t>(info["userInfoScale"].get_integer(100)),
				static_cast<int16_t>(info["userInfoDiffX"].get_integer(0)),
				static_cast<int16_t>(info["userInfoDiffY"].get_integer(0))
			});
			float scale = shown.scale / 100.0f;

			rider_args = DrawArgument(RIDER + Point<int16_t>(shown.x, shown.y), scale, scale);
		}

		buttons[Buttons::BT_TAMING_SHOW]->set_state(Button::State::NORMAL);
		set_mode(Mode::TAMING);
	}

	void UICharInfo::update_pets(int32_t character_id, const std::vector<Pet> &pets)
	{
		if (character_id != cid)
			return;

		pet_pages.clear();

		for (size_t i = 0; i < pets.size() && i < MAX_PETS; i++)
		{
			const Pet &pet = pets[i];
			PetPage page;
			page.look = PetLook(PetEntry{ pet.itemid, pet.name, 0, Point<int16_t>(), 0, 0 }, false);
			page.look.set_wear(pet.wear);
			page.name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE, pet.name);
			page.type = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(pet.itemid).get_name());
			page.level = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, std::to_string(pet.level));
			page.closeness = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, std::to_string(pet.closeness));
			page.fullness = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, std::to_string(pet.fullness));
			page.items = pets::info_items(i, pet.wear, look.get_pet_parts());
			pet_pages.push_back(std::move(page));
		}

		if (pet_pages.empty())
			return;

		buttons[Buttons::BT_PET_SHOW]->set_state(Button::State::NORMAL);
		select_pet(0);
		set_mode(Mode::PET);
	}

	void UICharInfo::update_wishes(int32_t character_id, const std::vector<int32_t> &sns)
	{
		if (character_id != cid)
			return;

		wish_rows.clear();
		wishes = char_info::wishes(sns, commodity);

		for (const char_info::Wish &wish : wishes)
		{
			const ItemData &item = ItemData::get(wish.item_id);
			wish_rows.push_back(panel_row(wish.item_id, char_info::wish_name(item.get_name(), wish.count), false));
		}

		// v83 lists the equipment again on each CHAR_INFO.
		list_worn();
		set_panel(panel);
	}

	void UICharInfo::list_worn()
	{
		std::vector<int32_t> items;
		Player &player = Stage::get().get_player();

		// One's own from the equipment worn, another's from the look they came with.
		if (cid == player.get_oid())
		{
			LookEntry entry = player.look_entry();
			items = char_info::worn_items(entry.equips, entry.maskedequips);
		}
		else if (auto character = Stage::get().get_character(cid))
		{
			items = character->get_look().get_info_items();
		}

		worn_rows.clear();

		for (int32_t itemid : items)
			worn_rows.push_back(panel_row(itemid, ItemData::get(itemid).get_name(), true));
	}

	UICharInfo::ItemRow UICharInfo::panel_row(int32_t itemid, const std::string &itemname, bool equip)
	{
		ItemRow row;
		row.icon = ItemData::get(itemid).get_icon(false);
		row.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, itemname);
		row.equip = equip;

		if (equip)
			row.level = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
							 std::to_string(EquipData::get(itemid).get_reqstat(MapleStat::Id::LEVEL)));

		return row;
	}

	void UICharInfo::update_collection(int32_t character_id, const Book &book, int32_t medal, const std::vector<int16_t> &quests)
	{
		if (character_id != cid)
			return;

		auto icon = char_info::level_icon(book.level, level_icons.size());
		level_icon = icon ? level_icons[*icon] : Texture();
		book_level.change_text(std::to_string(book.level));
		cover_name.change_text(book.cover != 0 ? std::string(nl::nx::string["Mob.img"][std::to_string(book.cover)]["name"]) : "");
		book_total.change_text(std::to_string(book.total));
		book_normal.change_text(std::to_string(book.normal));
		book_special.change_text(std::to_string(book.special));

		medal_icon = medal != 0 ? ItemData::get(medal).get_icon(false) : Texture();
		medal_name.change_text(medal != 0 ? ItemData::get(medal).get_name() : "");
		medal_count.change_text(std::to_string(quests.size()));

		medal_quests = quests;
		medal_offset = 0;
		medal_slider.setrows(0, MEDAL_ROWS, static_cast<int16_t>(medal_quests.size()));
		show_medals();
	}

	void UICharInfo::show_medals()
	{
		auto medal = [](int16_t quest)
		{
			return static_cast<int32_t>(nl::nx::quest["QuestInfo.img"][std::to_string(quest)]["viewMedalItem"].get_integer());
		};

		medal_titles.clear();

		for (int32_t item : char_info::medal_rows(medal_quests, static_cast<size_t>(std::max<int16_t>(medal_offset, 0)), medal))
			medal_titles.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
									  MedalTagLayout::text(ItemData::get(item).get_name()));
	}

	void UICharInfo::select_pet(size_t index)
	{
		selected = index;
		item_rows.clear();

		for (int32_t itemid : pet_pages[index].items)
			item_rows.push_back(item_row(itemid));

		item_offset = 0;
		item_slider.setrows(0, ITEM_ROWS, static_cast<int16_t>(item_rows.size()));
	}

	UICharInfo::ItemRow UICharInfo::item_row(int32_t itemid)
	{
		const ItemData &item = ItemData::get(itemid);
		const EquipData &equip = EquipData::get(itemid);

		ItemRow row;
		row.icon = item.get_icon(false);
		row.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, item.get_name());
		string_format::format_with_ellipsis(row.name, ITEM_NAME_WIDTH);
		row.equip = equip.is_valid();
		row.level = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						 std::to_string(equip.get_reqstat(MapleStat::Id::LEVEL)));

		return row;
	}

	bool UICharInfo::shows_mount() const
	{
		int32_t mountid = look.get_equips().get_equip(EquipSlot::Id::TAMEDMOB);
		int32_t saddleid = look.get_equips().get_equip(EquipSlot::Id::SADDLE);

		if (mountid <= 0 || saddleid <= 0)
			return true;

		return TamingMob::saddle_fits(saddleid, mountid);
	}

	void UICharInfo::change_fame(int16_t f)
	{
		fame.change_text(std::to_string(f));
	}

	bool UICharInfo::has_exceptions() const
	{
		return exceptions != nullptr;
	}

	void UICharInfo::add_exception(int32_t itemid)
	{
		if (exceptions)
			exceptions->add(itemid);
	}

	void UICharInfo::remove_exception(int32_t itemid)
	{
		if (exceptions)
			exceptions->remove(itemid);
	}

	void UICharInfo::toggle_exceptions()
	{
		if (exceptions)
			close_exceptions();
		else
			exceptions = std::make_unique<UIPetExceptions>(static_cast<uint8_t>(selected), exceptions_position());
	}

	void UICharInfo::close_exceptions()
	{
		if (!exceptions)
			return;

		exceptions->close();
		exceptions.reset();
	}

	Point<int16_t> UICharInfo::exceptions_position() const
	{
		return position + Point<int16_t>(EXCEPTIONS.x(), mode == Mode::MAIN ? EXCEPTIONS.y() : EXCEPTIONS_PET_Y);
	}

	bool UICharInfo::on_exceptions(Point<int16_t> cursorpos) const
	{
		if (!exceptions || Rectangle<int16_t>(position, position + dimension).contains(cursorpos))
			return false;

		return exceptions->is_in_range(cursorpos);
	}

	bool UICharInfo::can_show_exceptions() const
	{
		Player &player = Stage::get().get_player();

		return mode == Mode::PET && cid == player.get_oid() && player.get_pet(0)
			   && player.pet_wears(0, pets::ITEM_IGNORE);
	}

	void UICharInfo::close()
	{
		close_exceptions();
		deactivate();
	}

	void UICharInfo::invite() const
	{
		// Only a party's leader invites from here; anyone else's click does
		// nothing (FUN_008ff5b0).
		const Player &player = Stage::get().get_player();
		const Party &party = player.get_party();

		if (party.exists() && party.is_leader(player.get_oid()) && !name.get_text().empty())
			PartyCommands::invite(name.get_text());
	}

	void UICharInfo::give_fame(bool up) const
	{
		const char *refusal = nullptr;

		if (!Stage::get().get_character(cid))
			refusal = "Unable to find the character.";
		else if (!Stage::get().try_give_fame())
			refusal = "Please try it again later.";

		if (refusal)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(refusal, UIChatBar::LineType::SKILL);

			return;
		}

		GiveFamePacket(cid, up).dispatch();
	}

	void UICharInfo::set_mode(Mode m)
	{
		mode = m;
		dimension = Point<int16_t>(WIDTH, HEIGHTS[mode]);

		int16_t bottom = BOTTOMS[mode];

		struct Toggle
		{
			Mode target;
			int16_t x;
			uint16_t show, hide;
		};

		const Toggle toggles[] = {
			{ Mode::PET, PET_X, Buttons::BT_PET_SHOW, Buttons::BT_PET_HIDE },
			{ Mode::TAMING, TAMING_X, Buttons::BT_TAMING_SHOW, Buttons::BT_TAMING_HIDE },
			{ Mode::COLLECTION, COLLECTION_X, Buttons::BT_COLLECTION_SHOW, Buttons::BT_COLLECTION_HIDE }
		};

		for (auto [target, x, show, hide] : toggles)
		{
			buttons[show]->set_position(Point<int16_t>(x, bottom));
			buttons[hide]->set_position(Point<int16_t>(x, bottom));
			buttons[show]->set_active(mode != target);
			buttons[hide]->set_active(mode == target);
		}

		buttons[Buttons::BT_EXCEPTION]->set_position(Point<int16_t>(EXCEPTION_X, bottom));

		// The pet buttons show in the pet box, for the pets there are.
		for (size_t i = 0; i < MAX_PETS; i++)
		{
			auto &button = buttons[Buttons::BT_PET1 + i];
			button->set_active(mode == Mode::PET);
			button->set_state(i < pet_pages.size() ? Button::State::NORMAL : Button::State::DISABLED);
		}
	}

	void UICharInfo::set_panel(Panel p)
	{
		panel = p;
		buttons[Buttons::BT_PANEL_HIDE]->set_active(panel != Panel::NO_PANEL);

		panel_offset = 0;
		panel_slider.setrows(0, PANEL_ROWS, static_cast<int16_t>(panel_rows().size()));
		show_presents();
	}

	void UICharInfo::show_presents()
	{
		bool own = cid == Stage::get().get_player().get_oid();

		for (int16_t r = 0; r < PANEL_ROWS; r++)
		{
			Button &button = *buttons[Buttons::BT_PRESENT1 + r];
			size_t i = static_cast<size_t>(panel_offset + r);
			bool gives = panel_offset + r >= 0 && i < wishes.size() && char_info::giftable(wishes[i].item_id, own);

			button.set_active(panel == Panel::WISHES);
			button.set_state(gives ? Button::State::NORMAL : Button::State::DISABLED);
		}
	}

	const std::vector<UICharInfo::ItemRow> &UICharInfo::panel_rows() const
	{
		return panel == Panel::WISHES ? wish_rows : worn_rows;
	}

	bool UICharInfo::on_panel(Point<int16_t> cursorpos) const
	{
		Point<int16_t> panelpos = position + PANEL;

		return panel != Panel::NO_PANEL && !Rectangle<int16_t>(position, position + dimension).contains(cursorpos)
			   && Rectangle<int16_t>(panelpos, panelpos + PANEL_SIZE).contains(cursorpos);
	}
}
