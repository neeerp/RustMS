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
#include "UIRaceSelect.h"
#include "UIAranCreation.h"
#include "UICharSelect.h"
#include "UICygnusCreation.h"
#include "UIExplorerCreation.h"
#include "UIWorldSelect.h"

#include "../LoginScene.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"

#include "../../Audio/Audio.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <cctype>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Positions on screen with the camera at the race select stop,
		// measured against footage from 2009 (v80).
		const Point<int16_t> TITLE_POS = Point<int16_t>(406, 51);
		const Point<int16_t> TEXT_POS = Point<int16_t>(118, 291);
		const Point<int16_t> SELECT_POS = Point<int16_t>(566, 416);

		// Cygnus Knights, Explorers, and Aran, left to right.
		const Point<int16_t> RACE_POS[3] = {
			Point<int16_t>(104, 59), Point<int16_t>(293, 59), Point<int16_t>(511, 60)
		};

		// Login.img/RaceSelect's folder and button for each race.
		const char *RACE_NODE[3] = { "knight", "normal", "aran" };
		const char *RACE_BUTTON[3] = { "BtKnight", "BtNormal", "BtAran" };

		// The selected world box, as on the character select.
		const Point<int16_t> WORLD_BOX_POS = Point<int16_t>(0, 95);
		const Point<int16_t> WORLD_NAME_POS = Point<int16_t>(100, 103);
		const Point<int16_t> CHANNEL_NAME_POS = Point<int16_t>(100, 118);
	}

	UIRaceSelect::UIRaceSelect() : UIElement(Point<int16_t>(0, 0), Point<int16_t>(800, 600)), selected(Race::EXPLORER)
	{
		LoginScene::get().move_to(LoginScene::Step::RACE_SELECT);

		nl::node login = NxFiles::ui_classic["Login.img"];
		nl::node raceselect = login["RaceSelect"];

		title = raceselect["textGL"]["0"];

		for (uint8_t i = 0; i < Race::RACE_COUNT; i++)
		{
			nl::node race = raceselect[RACE_NODE[i]];

			buttons[Buttons::BT_CYGNUS + i] = std::make_unique<MapleButton>(race[RACE_BUTTON[i]], RACE_POS[i]);
			race_on[i] = race["OnAnimation"]["0"];
			race_text[i] = race["text"]["0"];
		}

		buttons[Buttons::BT_SELECT] = std::make_unique<MapleButton>(raceselect["BtSelect"], SELECT_POS);
		buttons[Buttons::BT_VIEWALLCHAR] = std::make_unique<MapleButton>(login["ViewAllChar"]["BtVAC"], Point<int16_t>(0, 381));
		buttons[Buttons::BT_BACK] = std::make_unique<MapleButton>(login["Common"]["BtStart"], Point<int16_t>(0, 433));

		buttons[Buttons::BT_VIEWALLCHAR]->set_state(Button::State::DISABLED);

		world_box = login["Common"]["selectWorld"];

		std::string world;
		uint8_t world_id = Configuration::get().get_worldid();
		uint8_t channel_id = Configuration::get().get_channelid();

		if (auto worldselect = UI::get().get_element<UIWorldSelect>())
			world = worldselect->get_world_name(world_id);

		world_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY, world);
		channel_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY,
			"CH." + std::to_string(channel_id + 1));
	}

	void UIRaceSelect::draw(float inter) const
	{
		title.draw(position + TITLE_POS);
		race_text[selected].draw(position + TEXT_POS);

		UIElement::draw(inter);

		// The picked race is drawn in colour over its button, which keeps
		// the name plate under it.
		race_on[selected].draw(position + RACE_POS[selected]);

		// The selected world box sits over the Cygnus Knights.
		world_box.draw(position + WORLD_BOX_POS);
		world_name.draw(position + WORLD_NAME_POS);
		channel_name.draw(position + CHANNEL_NAME_POS);
	}

	void UIRaceSelect::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			show_charselect();
		else if (keycode == KeyAction::Id::LEFT && selected > 0)
			select_race(selected - 1);
		else if (keycode == KeyAction::Id::RIGHT && selected + 1 < Race::RACE_COUNT)
			select_race(selected + 1);
		else if (keycode == KeyAction::Id::RETURN)
			make_character();
	}

	UIElement::Type UIRaceSelect::get_type() const
	{
		return TYPE;
	}

	bool UIRaceSelect::check_name(std::string name) const
	{
		nl::node ForbiddenName = nl::nx::etc["ForbiddenName.img"];

		for (auto forbiddenName : ForbiddenName)
		{
			std::string lName = to_lower(name);
			std::string fName = to_lower(forbiddenName);

			if (lName.find(fName) != std::string::npos)
				return false;
		}

		return true;
	}

	void UIRaceSelect::send_naming_result(bool nameused)
	{
		switch (selected)
		{
		case Race::CYGNUS:
			if (auto cygnuscreation = UI::get().get_element<UICygnusCreation>())
				cygnuscreation->send_naming_result(nameused);

			break;
		case Race::EXPLORER:
			if (auto explorercreation = UI::get().get_element<UIExplorerCreation>())
				explorercreation->send_naming_result(nameused);

			break;
		case Race::ARAN:
			if (auto arancreation = UI::get().get_element<UIAranCreation>())
				arancreation->send_naming_result(nameused);

			break;
		}
	}

	Button::State UIRaceSelect::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_CYGNUS:
		case Buttons::BT_EXPLORER:
		case Buttons::BT_ARAN:
			select_race(buttonid - Buttons::BT_CYGNUS);
			break;
		case Buttons::BT_SELECT:
			make_character();
			break;
		case Buttons::BT_BACK:
			show_charselect();
			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	void UIRaceSelect::select_race(uint8_t race)
	{
		if (race == selected)
			return;

		Sound(Sound::Name::RACESELECT).play();

		selected = race;
	}

	void UIRaceSelect::make_character()
	{
		Sound(Sound::Name::SCROLLUP).play();

		deactivate();

		switch (selected)
		{
		case Race::CYGNUS:
			UI::get().emplace<UICygnusCreation>();
			break;
		case Race::EXPLORER:
			UI::get().emplace<UIExplorerCreation>();
			break;
		case Race::ARAN:
			UI::get().emplace<UIAranCreation>();
			break;
		}
	}

	void UIRaceSelect::show_charselect()
	{
		Sound(Sound::Name::SCROLLUP).play();

		deactivate();

		if (auto charselect = UI::get().get_element<UICharSelect>())
		{
			charselect->makeactive();
			LoginScene::get().move_to(LoginScene::Step::CHAR_SELECT);
		}
	}

	std::string UIRaceSelect::to_lower(std::string value) const
	{
		std::transform(value.begin(), value.end(), value.begin(), ::tolower);

		return value;
	}
}
