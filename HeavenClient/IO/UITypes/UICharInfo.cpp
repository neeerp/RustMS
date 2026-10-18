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

#include "../../Character/Job.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UITradingRoom.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

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
		const Point<int16_t> FAMILY = { 105, 23 };
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
		buttons[Buttons::BT_FAMILY] = std::make_unique<MapleButton>(UserInfo["BtFamily"], FAMILY);
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

		// There are no families, parties, or pets yet.
		for (auto button : { Buttons::BT_FAMILY, Buttons::BT_PARTY, Buttons::BT_EXCEPTION })
			buttons[button]->set_state(Button::State::DISABLED);

		// v83 offers fame only from level 15 (FUN_009007a0), and never for
		// oneself (FUN_00903b86).
		auto &player = Stage::get().get_player();

		if (cid == player.get_oid() || player.get_stats().get_stat(MapleStat::Id::LEVEL) < 15)
			for (auto button : { Buttons::BT_FAME_UP, Buttons::BT_FAME_DOWN })
				buttons[button]->set_state(Button::State::DISABLED);

		name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);
		level = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		job = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		fame = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		guild = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		alliance = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		// Copy what is shown: the character may leave the map while the
		// window is open.
		if (auto character = Stage::get().get_character(cid))
		{
			look = character->get_look();
			name.change_text(character->get_name());
			known = true;
		}

		set_mode(Mode::MAIN);
		set_panel(Panel::NO_PANEL);
	}

	void UICharInfo::draw(float alpha) const
	{
		backgrounds[mode].draw(position);

		if (panel == Panel::ITEMS)
			item_list.draw(position + PANEL);
		else if (panel == Panel::WISHES)
			wish_list.draw(position + PANEL);

		if (known)
			look.draw(position + PORTRAIT, false, Stance::Id::STAND1, Expression::Id::DEFAULT);

		name.draw(position + NAME);

		const Text *rows[] = { &level, &job, &fame, &guild, &alliance };

		for (size_t i = 0; i < 5; i++)
			rows[i]->draw(position + Point<int16_t>(VALUE_X, ROW_LEVEL + ROW_STEP * i + 4 + TEXT_Y));

		UIElement::draw_buttons(alpha);
	}

	Button::State UICharInfo::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_CLOSE:
			deactivate();
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
		case Buttons::BT_FAME_UP:
			give_fame(true);
			break;
		case Buttons::BT_FAME_DOWN:
			give_fame(false);
			break;
		case Buttons::BT_TRADE:
			UITradingRoom::request(cid);
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
		case Buttons::BT_TAMING_HIDE:
		case Buttons::BT_COLLECTION_HIDE:
			set_mode(Mode::MAIN);
			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	bool UICharInfo::is_in_range(Point<int16_t> cursorpos) const
	{
		if (Rectangle<int16_t>(position, position + dimension).contains(cursorpos))
			return true;

		Point<int16_t> panelpos = position + PANEL;

		return panel != Panel::NO_PANEL && Rectangle<int16_t>(panelpos, panelpos + PANEL_SIZE).contains(cursorpos);
	}

	void UICharInfo::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			deactivate();
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

	void UICharInfo::change_fame(int16_t f)
	{
		fame.change_text(std::to_string(f));
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
	}

	void UICharInfo::set_panel(Panel p)
	{
		panel = p;
		buttons[Buttons::BT_PANEL_HIDE]->set_active(panel != Panel::NO_PANEL);
	}
}
