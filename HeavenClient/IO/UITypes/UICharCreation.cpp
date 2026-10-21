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
#include "UICharCreation.h"

#include "UICharSelect.h"
#include "UILoginNotice.h"
#include "UIRaceSelect.h"

#include "../LoginScene.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"
#include "../../Constants.h"

#include "../../Audio/Audio.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Util/CharacterName.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/CharCreationPackets.h"

#include <algorithm>
#include <cctype>

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where a race's boards and buttons sit on screen with the camera at
		// its stop. Explorers and Aran are measured against footage from 2009
		// (v73 and v80); the Cygnus boards have the Explorer shapes, so they
		// hang in the same places with their buttons centred on the plank.
		struct Layout
		{
			const char *node;
			Point<int16_t> name_board;
			Point<int16_t> name_ok;
			Point<int16_t> name_cancel;
			Rectangle<int16_t> name_slot;
			Color::Name name_color;
			Point<int16_t> setting_board;
			Point<int16_t> setting_ok;
			Point<int16_t> setting_cancel;
			Point<int16_t> first_row;
		};

		const Layout LAYOUTS[UICharCreation::Race::RACE_COUNT] = {
			{
				"NewCharKnight",
				Point<int16_t>(478, 102), Point<int16_t>(512, 278), Point<int16_t>(585, 278),
				Rectangle<int16_t>(512, 654, 211, 229), Color::Name::WHITE,
				Point<int16_t>(469, 102), Point<int16_t>(516, 432), Point<int16_t>(589, 432),
				Point<int16_t>(482, 212)
			},
			{
				"NewChar",
				Point<int16_t>(478, 102), Point<int16_t>(505, 275), Point<int16_t>(579, 275),
				Rectangle<int16_t>(512, 654, 209, 231), Color::Name::WHITE,
				Point<int16_t>(469, 102), Point<int16_t>(507, 431), Point<int16_t>(580, 431),
				Point<int16_t>(481, 204)
			},
			{
				"NewCharAran",
				Point<int16_t>(476, 106), Point<int16_t>(510, 244), Point<int16_t>(584, 244),
				Rectangle<int16_t>(510, 656, 201, 223), Color::Name::BLACK,
				Point<int16_t>(443, 105), Point<int16_t>(507, 370), Point<int16_t>(581, 370),
				Point<int16_t>(472, 208)
			}
		};

		// Etc.img/MakeCharInfo's lists for each race, and the folder in its
		// Name list that names the hair, hair colours, and skins. Aran
		// shares the Explorer names.
		const char *LOOK_NODE[UICharCreation::Race::RACE_COUNT] = { "PremiumChar", "Char", "OrientChar" };
		const char *NAME_NODE[UICharCreation::Race::RACE_COUNT] = { "PremiumChar", "Char", "Char" };

		// The new character stands where the scene's platform is.
		const Point<int16_t> CHAR_POS = Point<int16_t>(394, 339);

		// Within a Character Setting row: the arrows close in its value
		// cell, and the value is centred between them.
		const int16_t ROW_HEIGHT = 18;
		const Point<int16_t> LEFT_ARROW = Point<int16_t>(57, 1);
		const Point<int16_t> RIGHT_ARROW = Point<int16_t>(184, 1);
		const Point<int16_t> VALUE_POS = Point<int16_t>(128, -2);
		// A longer name, like the Aran's Average Musashi Shoes, takes the
		// narrower font to fit between the arrows.
		const int16_t VALUE_WIDTH = 112;

		const char *GENDER_NAME[2] = { "Male", "Female" };
	}

	UICharCreation::UICharCreation(uint8_t r) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(800, 600)), race(r), settings(false), female(false)
	{
		LoginScene::get().move_to(static_cast<LoginScene::Step>(LoginScene::Step::CYGNUS_CREATION + race));

		const Layout &layout = LAYOUTS[race];

		nl::node login = NxFiles::ui_classic["Login.img"];
		nl::node newchar_node = login[layout.node];

		name_board = newchar_node["charName"];
		setting_board = newchar_node["charSet"];

		for (uint8_t row = 0; row < Row::ROW_COUNT; row++)
		{
			rows[row] = newchar_node["avatarSel"][std::to_string(row)]["normal"];

			Point<int16_t> row_pos = layout.first_row + Point<int16_t>(0, ROW_HEIGHT * row);

			buttons[Buttons::BT_ARROWS + row * 2] = std::make_unique<MapleButton>(newchar_node["BtLeft"], row_pos + LEFT_ARROW);
			buttons[Buttons::BT_ARROWS + row * 2 + 1] = std::make_unique<MapleButton>(newchar_node["BtRight"], row_pos + RIGHT_ARROW);
		}

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(newchar_node["BtYes"]);
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(newchar_node["BtNo"]);
		buttons[Buttons::BT_VIEWALLCHAR] = std::make_unique<MapleButton>(login["ViewAllChar"]["BtVAC"], Point<int16_t>(0, 381));
		buttons[Buttons::BT_BACK] = std::make_unique<MapleButton>(login["Common"]["BtStart"], Point<int16_t>(0, 433));

		buttons[Buttons::BT_VIEWALLCHAR]->set_state(Button::State::DISABLED);

		name = Textfield(Text::Font::A12M, Text::Alignment::LEFT, layout.name_color, layout.name_slot, 12);

		// Enter on an empty name does nothing; OK says it cannot be used
		name.set_enter_callback(
			[&](std::string text)
			{
				if (!text.empty())
					button_pressed(Buttons::BT_OK);
			}
		);

		name.set_key_callback(
			KeyAction::Id::ESCAPE,
			[&]()
			{
				button_pressed(Buttons::BT_CANCEL);
			}
		);

		nl::node info = nl::nx::etc["MakeCharInfo.img"];

		if (race == Race::EXPLORER)
			info = info["Info"];

		for (uint8_t gender = 0; gender < 2; gender++)
		{
			nl::node looks = info[std::string(LOOK_NODE[race]) + (gender ? "Female" : "Male")];

			for (uint8_t row = 0; row < Row::GENDER; row++)
			{
				for (auto choice : looks[std::to_string(row)])
				{
					int32_t value = choice;
					choices[gender][row].push_back(value);
				}
			}
		}

		newchar.set_direction(true);

		reset_look();
		show_settings(false);
	}

	void UICharCreation::draw(float inter) const
	{
		const Layout &layout = LAYOUTS[race];

		if (settings)
		{
			setting_board.draw(position + layout.setting_board);

			for (uint8_t row = 0; row < Row::ROW_COUNT; row++)
			{
				Point<int16_t> row_pos = position + layout.first_row + Point<int16_t>(0, ROW_HEIGHT * row);

				rows[row].draw(row_pos);
				values[row].draw(row_pos + VALUE_POS);
			}
		}
		else
		{
			name_board.draw(position + layout.name_board);
			name.draw(position);
		}

		UIElement::draw(inter);

		newchar.draw(position + CHAR_POS, inter);
	}

	void UICharCreation::update()
	{
		UIElement::update();

		if (!settings)
			name.update(position);

		newchar.update(Constants::TIMESTEP);
	}

	Cursor::State UICharCreation::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (!settings && name.get_state() == Textfield::State::NORMAL && name.get_bounds().contains(cursorpos))
		{
			if (clicked)
			{
				name.set_state(Textfield::State::FOCUSED);

				return Cursor::State::CLICKING;
			}

			return Cursor::State::IDLE;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICharCreation::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			button_pressed(Buttons::BT_CANCEL);
		else if (keycode == KeyAction::Id::RETURN && (settings || !name.get_text().empty()))
			button_pressed(Buttons::BT_OK);
	}

	UIElement::Type UICharCreation::get_type() const
	{
		return TYPE;
	}

	void UICharCreation::send_naming_result(uint8_t result)
	{
		if (settings)
			return;

		if (result == 0)
		{
			enable_buttons(true);
			show_settings(true);

			return;
		}

		// v83 (FUN_005f9c72): 1 is a taken name, 2 one the server refuses
		UILoginNotice::Message message = UILoginNotice::Message::UNKNOWN_ERROR;

		if (result == 1)
			message = UILoginNotice::Message::NAME_IN_USE;
		else if (result == 2)
			message = UILoginNotice::Message::ILLEGAL_NAME;

		UI::get().emplace<UILoginNotice>(
			message,
			[&]()
			{
				enable_buttons(true);
				name.set_state(Textfield::State::FOCUSED);
			}
		);
	}

	void UICharCreation::send_creation_failed(uint8_t result)
	{
		// 0x1A, a pending world transfer, has its own text in v83; the
		// server never sends it.
		UILoginNotice::Message message = UILoginNotice::Message::UNKNOWN_ERROR;

		if (result == 0x0A)
			message = UILoginNotice::Message::TOO_MANY_REQUESTS;
		else if (result == 0x1E)
			message = UILoginNotice::Message::ILLEGAL_NAME;

		UI::get().emplace<UILoginNotice>(message, [&]() { enable_buttons(true); });
	}

	void UICharCreation::send_created()
	{
		Sound(Sound::Name::SCROLLUP).play();

		UI::get().remove(UIElement::Type::CLASSCREATION);
		UI::get().remove(UIElement::Type::RACESELECT);

		if (auto charselect = UI::get().get_element<UICharSelect>())
			charselect->post_add_character();
	}

	Button::State UICharCreation::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_OK:
			if (settings)
				create_character();
			else
				check_name();

			break;
		case Buttons::BT_CANCEL:
			if (settings)
				show_settings(false);
			else
				show_raceselect();

			break;
		case Buttons::BT_BACK:
			show_raceselect();
			break;
		default:
			if (buttonid >= Buttons::BT_ARROWS)
			{
				uint16_t arrow = buttonid - Buttons::BT_ARROWS;

				step(arrow / 2, arrow % 2);
			}

			break;
		}

		return Button::State::NORMAL;
	}

	void UICharCreation::show_settings(bool s)
	{
		settings = s;

		const Layout &layout = LAYOUTS[race];

		buttons[Buttons::BT_OK]->set_position(settings ? layout.setting_ok : layout.name_ok);
		buttons[Buttons::BT_CANCEL]->set_position(settings ? layout.setting_cancel : layout.name_cancel);

		for (uint16_t arrow = 0; arrow < Row::ROW_COUNT * 2; arrow++)
			buttons[Buttons::BT_ARROWS + arrow]->set_active(settings);

		name.set_state(settings ? Textfield::State::DISABLED : Textfield::State::FOCUSED);
	}

	void UICharCreation::check_name()
	{
		std::string text = name.get_text();

		enable_buttons(false);
		name.set_state(Textfield::State::DISABLED);

		if (name_allowed(text))
		{
			NameCharPacket(text).dispatch();

			return;
		}

		UI::get().emplace<UILoginNotice>(
			UILoginNotice::Message::ILLEGAL_NAME,
			[&]()
			{
				enable_buttons(true);
				name.set_state(Textfield::State::FOCUSED);
			}
		);
	}

	void UICharCreation::create_character()
	{
		enable_buttons(false);

		// The server adds the style and colour digit itself. The race is the
		// class flag the server turns into a starting job.
		CreateCharPacket(
			name.get_text(), race,
			choices[female][Row::FACE][chosen[Row::FACE]],
			choices[female][Row::HAIR][chosen[Row::HAIR]],
			static_cast<uint8_t>(choices[female][Row::HAIR_COLOR][chosen[Row::HAIR_COLOR]]),
			static_cast<uint8_t>(choices[female][Row::SKIN][chosen[Row::SKIN]]),
			choices[female][Row::TOP][chosen[Row::TOP]],
			choices[female][Row::BOTTOM][chosen[Row::BOTTOM]],
			choices[female][Row::SHOES][chosen[Row::SHOES]],
			choices[female][Row::WEAPON][chosen[Row::WEAPON]],
			female
		).dispatch();
	}

	void UICharCreation::show_raceselect()
	{
		Sound(Sound::Name::SCROLLUP).play();

		if (auto raceselect = UI::get().get_element<UIRaceSelect>())
		{
			raceselect->makeactive();
			LoginScene::get().move_to(LoginScene::Step::RACE_SELECT);
		}

		UI::get().remove(UIElement::Type::CLASSCREATION);
	}

	void UICharCreation::step(uint8_t row, bool right)
	{
		if (row == Row::GENDER)
		{
			female = !female;
			reset_look();

			return;
		}

		size_t count = choices[female][row].size();

		if (count == 0)
			return;

		chosen[row] = right ? (chosen[row] + 1) % count : (chosen[row] + count - 1) % count;

		update_look();
	}

	void UICharCreation::reset_look()
	{
		std::fill(std::begin(chosen), std::end(chosen), 0);

		update_look();
	}

	void UICharCreation::update_look()
	{
		auto value = [&](uint8_t row)
		{
			const std::vector<int32_t> &list = choices[female][row];

			return list.empty() ? 0 : list[chosen[row]];
		};

		newchar.set_body(value(Row::SKIN));
		newchar.set_face(value(Row::FACE));
		newchar.set_hair(value(Row::HAIR) + value(Row::HAIR_COLOR));

		for (uint8_t row = Row::TOP; row <= Row::WEAPON; row++)
			if (int32_t equip = value(row))
				newchar.add_equip(equip);

		for (uint8_t row = 0; row < Row::ROW_COUNT; row++)
		{
			std::string text = row_text(row);
			values[row] = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::BLACK, text);

			if (values[row].width() > VALUE_WIDTH)
				values[row] = Text(Text::Font::A10M, Text::Alignment::CENTER, Color::Name::BLACK, text);
		}
	}

	void UICharCreation::enable_buttons(bool enabled)
	{
		Button::State state = enabled ? Button::State::NORMAL : Button::State::DISABLED;

		buttons[Buttons::BT_OK]->set_state(state);
		buttons[Buttons::BT_CANCEL]->set_state(state);
	}

	std::string UICharCreation::row_text(uint8_t row) const
	{
		if (row == Row::GENDER)
			return GENDER_NAME[female];

		const std::vector<int32_t> &list = choices[female][row];

		if (list.empty())
			return "";

		return choice_name(row, list[chosen[row]]);
	}

	std::string UICharCreation::choice_name(uint8_t row, int32_t value) const
	{
		switch (row)
		{
		case Row::FACE:
			return newchar.get_face()->get_name();
		case Row::HAIR:
		case Row::HAIR_COLOR:
		case Row::SKIN:
		{
			std::string folder = std::string(NAME_NODE[race]) + (female ? "Female" : "Male");
			nl::node text = nl::nx::etc["MakeCharInfo.img"]["Name"][folder][std::to_string(row)][std::to_string(value)];

			if (text)
				return text;

			if (row == Row::HAIR)
				return newchar.get_hair()->get_name();

			return newchar.get_body()->get_name();
		}
		default:
			return ItemData::get(value).get_name();
		}
	}

	// v83's name rules (FUN_0079fc74), and for everyone but GMs no word from
	// ForbiddenName.img or, up to a comma, Curse.img anywhere in the name
	bool UICharCreation::name_allowed(const std::string &text) const
	{
		if (!character_name_valid(text))
			return false;

		return Configuration::get().get_admin() || !CurseData::in_name(text);
	}
}
