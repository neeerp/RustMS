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
#include "UICharSelect.h"
#include "UILoginNotice.h"
#include "UIRaceSelect.h"
#include "UISoftKey.h"
#include "UIWorldSelect.h"

#include "../LoginScene.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"

#include "../../Audio/Audio.h"
#include "../../Character/Job.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/SelectCharPackets.h"

namespace ms
{
	namespace
	{
		// Positions on screen with the camera at the character select stop,
		// measured against period footage.
		constexpr int16_t PAGESIZE = 3;

		// Where the first character's feet rest on the bridge.
		const Point<int16_t> SLOT_POS = Point<int16_t>(256, 368);
		constexpr int16_t SLOT_STEP = 125;

		// The area around a character's feet that picks it.
		constexpr int16_t SLOT_HALF_WIDTH = 30;
		constexpr int16_t SLOT_HEIGHT = 80;

		// Start, Create a character and Delete a character, in that order.
		const Point<int16_t> MENU_POS[3] = {
			Point<int16_t>(575, 140), Point<int16_t>(573, 180), Point<int16_t>(567, 222)
		};

		const Point<int16_t> PAGE_LEFT_POS = Point<int16_t>(150, 318);
		const Point<int16_t> PAGE_RIGHT_POS = Point<int16_t>(605, 318);

		const Point<int16_t> WORLD_BOX_POS = Point<int16_t>(0, 95);
		const Point<int16_t> WORLD_NAME_POS = Point<int16_t>(100, 103);
		const Point<int16_t> CHANNEL_NAME_POS = Point<int16_t>(100, 118);

		// The bottom centre of the balloon under the step sign.
		const Point<int16_t> BALLOON_POS = Point<int16_t>(380, 78);

		// From a character's feet: the top of the light that falls on it
		// when picked, and the top left of its stats scroll.
		const Point<int16_t> BEAM_OFFSET = Point<int16_t>(0, -337);
		const Point<int16_t> SCROLL_OFFSET = Point<int16_t>(-118, -233);

		// The stats table inside the open scroll, and its cells.
		const Point<int16_t> CHARINFO_OFFSET = Point<int16_t>(18, 25);
		constexpr int16_t VALUE_LEFT = 44;
		constexpr int16_t VALUE_RIGHT = 135;
		constexpr int16_t ROW_HEIGHT = 18;
		constexpr int16_t RANK_ROW = 97;
		constexpr int16_t JOB_RANK_HEADER = 116;
		constexpr int16_t JOB_RANK_ROW = 133;
		constexpr int16_t CHARINFO_WIDTH = 178;

		// The name tag's top edge is level with the feet and its centre sits
		// a little right of them. It fits the name with 9 px to spare on
		// each side, and is never narrower than 58 px. Its middle piece
		// repeats every 9 px from the left edge; the end pieces go on top.
		constexpr int16_t NAMETAG_X = 6;
		constexpr int16_t NAMETAG_MIN_NAME = 40;
		constexpr int16_t NAMETAG_PADDING = 18;
		constexpr int16_t NAMETAG_RIGHT = 10;
		// v83 draws the name 2 px below the tag's top; our text sits 4 px
		// lower than the original's for the same position, as in the table.
		constexpr int16_t NAMETAG_TEXT_Y = 2 - 4;

		Point<int16_t> slot_pos(int16_t slot)
		{
			return SLOT_POS + Point<int16_t>(SLOT_STEP * slot, 0);
		}

		// Login.img/CharSelect art comes in three sets: Explorers, Cygnus
		// Knights and Aran.
		uint8_t job_set(uint16_t job)
		{
			if (job >= 2000)
				return 2;

			if (job >= 1000)
				return 1;

			return 0;
		}

		// The job ranking's title art: beginner, warrior, magician, bowman,
		// thief. Pirates have none.
		int8_t job_icon(uint16_t job)
		{
			if (job % 1000 == 0 || job == 2000)
				return 0;

			if (job >= 2100 && job < 2200)
				return 1;

			uint16_t branch = (job % 1000) / 100;

			return branch <= 4 ? branch : -1;
		}

		bool has_rank(const StatsEntry &stats)
		{
			return stats.rank.second != 0;
		}

		std::string rank_line(const std::pair<int32_t, int8_t> &rank)
		{
			return "Ranked at " + std::to_string(rank.first);
		}
	}

	UICharSelect::UICharSelect(std::vector<CharEntry> c, int8_t char_count, int32_t s, int8_t rp) :
		UIElement(Point<int16_t>(0, 0), Point<int16_t>(800, 600)), characters(c), characters_count(char_count),
		slots(s), require_pic(rp)
	{
		LoginScene::get().move_to(LoginScene::Step::CHAR_SELECT);

		nl::node login = NxFiles::ui_classic["Login.img"];
		nl::node charselect = login["CharSelect"];

		buttons[Buttons::BT_SELECT] = std::make_unique<MapleButton>(charselect["BtSelect"], MENU_POS[0]);
		buttons[Buttons::BT_NEW] = std::make_unique<MapleButton>(charselect["BtNew"], MENU_POS[1]);
		buttons[Buttons::BT_DELETE] = std::make_unique<MapleButton>(charselect["BtDelete"], MENU_POS[2]);
		buttons[Buttons::BT_VIEWALLCHAR] = std::make_unique<MapleButton>(login["ViewAllChar"]["BtVAC"], Point<int16_t>(0, 381));
		buttons[Buttons::BT_BACK] = std::make_unique<MapleButton>(login["Common"]["BtStart"], Point<int16_t>(0, 433));

		// The server can't list every character.
		buttons[Buttons::BT_VIEWALLCHAR]->set_state(Button::State::DISABLED);

		focus_hand[0] = charselect["BtSelect"]["keyFocused"];
		focus_hand[1] = charselect["BtNew"]["keyFocused"];
		focus_hand[2] = charselect["BtDelete"]["keyFocused"];

		signposts[0] = charselect["adventure"]["0"];
		signposts[1] = charselect["knight"]["0"];
		signposts[2] = charselect["aran"]["0"];

		for (uint8_t i = 0; i < 2; i++)
			for (uint8_t j = 0; j < 3; j++)
				nametags[i][j] = charselect["nameTag"][std::to_string(i)][std::to_string(j)];

		beam = charselect["effect"]["1"];
		sparkle = charselect["effect"]["0"];

		// The short scroll holds one ranking, the long one two.
		scroll_open[0] = charselect["scroll"]["0"];
		scroll_open[1] = charselect["scroll"]["2"];
		scroll_opened[0] = charselect["scroll"]["0"]["3"];
		scroll_opened[1] = charselect["scroll"]["2"]["3"];
		charinfo[0] = charselect["charInfo"];
		charinfo[1] = charselect["charInfo1"];

		rank_icons[0] = charselect["icon"]["up"];
		rank_icons[1] = charselect["icon"]["down"];
		rank_icons[2] = charselect["icon"]["same"];

		for (uint8_t i = 0; i < 5; i++)
			job_icons[i] = charselect["icon"]["job"][std::to_string(i)];

		page_left.normal = charselect["pageL"]["0"]["0"];
		page_left.hovered = charselect["pageL"]["1"]["0"];
		page_left.position = PAGE_LEFT_POS;
		page_right.normal = charselect["pageR"]["0"]["0"];
		page_right.hovered = charselect["pageR"]["1"]["0"];
		page_right.position = PAGE_RIGHT_POS;

		world_box = login["Common"]["selectWorld"];

		std::string world;
		uint8_t world_id = Configuration::get().get_worldid();
		uint8_t channel_id = Configuration::get().get_channelid();

		if (auto worldselect = UI::get().get_element<UIWorldSelect>())
			world = worldselect->get_world_name(world_id);

		world_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY, world);
		channel_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY,
			"CH." + std::to_string(channel_id + 1));

		chatballoon.change_text("Use the Tab key to toggle the menu.");

		for (auto &entry : characters)
		{
			charlooks.emplace_back(entry.look);
			names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DUSTYGRAY, entry.stats.name);
		}

		page_count = std::max<int16_t>(1, (std::max<int32_t>(slots, characters_count) + PAGESIZE - 1) / PAGESIZE);

		int16_t last = Setting<DefaultCharacter>::get().load();

		// The saved slot may be from an account with more characters than this one.
		if (last >= characters_count)
			last = characters_count - 1;

		if (last >= 0)
		{
			page = last / PAGESIZE;
			select_character(last);
		}

		update_buttons();

		if (Configuration::get().get_auto_login())
		{
			SelectCharPicPacket(
					Configuration::get().get_auto_pic(),
					Configuration::get().get_auto_cid()
			).dispatch();
		}
	}

	void UICharSelect::draw(float alpha) const
	{
		world_box.draw(position + WORLD_BOX_POS);
		world_name.draw(position + WORLD_NAME_POS);
		channel_name.draw(position + CHANNEL_NAME_POS);

		bool on_page = selected >= 0 && selected / PAGESIZE == page;
		Point<int16_t> selected_pos = on_page ? position + slot_pos(selected % PAGESIZE) : Point<int16_t>();

		// The signposts stand behind the stats scroll.
		for (int16_t i = 0; i < PAGESIZE; i++)
		{
			int16_t index = page * PAGESIZE + i;

			if (index >= characters_count)
				break;

			uint16_t job = characters[index].stats.stats[MapleStat::Id::JOB];
			signposts[job_set(job)].draw(position + slot_pos(i));
		}

		if (on_page && !beam_done)
			beam.draw(selected_pos + BEAM_OFFSET, alpha);

		if (on_page && scroll != Scroll::CLOSED)
		{
			const StatsEntry &stats = characters[selected].stats;
			uint16_t job = stats.stats[MapleStat::Id::JOB];
			bool ranked = has_rank(stats);
			bool job_rank = ranked && job % 1000 != 0 && job != 2000;
			Point<int16_t> scrollpos = selected_pos + SCROLL_OFFSET;

			if (scroll == Scroll::OPENING)
			{
				scroll_open[job_rank].draw(scrollpos, alpha);
			} else
			{
				scroll_opened[job_rank].draw(scrollpos);

				Point<int16_t> table = scrollpos + CHARINFO_OFFSET;
				charinfo[job_rank].draw(table + charinfo[job_rank].get_origin());

				auto cell = [&](int16_t x, int16_t row, const std::string &text)
				{
					Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text)
						.draw(table + Point<int16_t>(x, ROW_HEIGHT * row - 4));
				};

				cell(VALUE_LEFT, 0, Job(job).get_name());
				cell(VALUE_LEFT, 1, std::to_string(stats.stats[MapleStat::Id::LEVEL]));
				cell(VALUE_RIGHT, 1, std::to_string(stats.stats[MapleStat::Id::FAME]));
				cell(VALUE_LEFT, 2, std::to_string(stats.stats[MapleStat::Id::STR]));
				cell(VALUE_RIGHT, 2, std::to_string(stats.stats[MapleStat::Id::INT]));
				cell(VALUE_LEFT, 3, std::to_string(stats.stats[MapleStat::Id::DEX]));
				cell(VALUE_RIGHT, 3, std::to_string(stats.stats[MapleStat::Id::LUK]));

				auto ranking = [&](int16_t y, const std::pair<int32_t, int8_t> &rank)
				{
					std::string line = ranked ? rank_line(rank) : "Ranking not available";
					Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK, line)
						.draw(table + Point<int16_t>(CHARINFO_WIDTH / 2, y - 4));

					if (!ranked)
						return;

					uint8_t icon = rank.second == '+' ? 0 : rank.second == '-' ? 1 : 2;
					rank_icons[icon].draw(table + Point<int16_t>(CHARINFO_WIDTH - 16, y + 4));
				};

				ranking(RANK_ROW, stats.rank);

				if (job_rank)
				{
					int8_t icon = job_icon(job);

					if (icon >= 0)
					{
						const Texture &title = job_icons[icon];
						title.draw(table + Point<int16_t>(80 - title.width(), JOB_RANK_HEADER + 4));
					}

					ranking(JOB_RANK_ROW, stats.jobrank);
				}
			}
		}

		for (int16_t i = 0; i < PAGESIZE; i++)
		{
			int16_t index = page * PAGESIZE + i;

			if (index >= characters_count)
				break;

			Point<int16_t> feet = position + slot_pos(i);

			charlooks[index].draw(DrawArgument(feet, false), alpha);

			const Texture *tag = nametags[index == selected];
			const Text &name = names[index];
			int16_t width = std::max(name.width(), NAMETAG_MIN_NAME) + NAMETAG_PADDING;
			Point<int16_t> left = feet + Point<int16_t>(NAMETAG_X - width / 2, 0);
			int16_t step = tag[1].width();

			for (int16_t x = 0; step > 0 && x < width - NAMETAG_RIGHT; x += step)
				tag[1].draw(left + Point<int16_t>(x, 0));

			tag[0].draw(left);
			tag[2].draw(left + Point<int16_t>(width - NAMETAG_RIGHT, 0));
			name.draw(left + Point<int16_t>((width - name.width()) / 2 - 1, NAMETAG_TEXT_Y));
		}

		if (on_page && !sparkle_done)
			sparkle.draw(selected_pos + BEAM_OFFSET, alpha);

		if (page_count > 1)
		{
			for (const PageArrow *arrow : { &page_left, &page_right })
				(arrow->hover ? arrow->hovered : arrow->normal).draw(position + arrow->position);
		}

		UIElement::draw_buttons(alpha);

		if (menu_focus >= 0)
			focus_hand[menu_focus].draw(position + MENU_POS[menu_focus], alpha);

		chatballoon.draw(position + BALLOON_POS);
	}

	void UICharSelect::update()
	{
		UIElement::update();

		for (auto &charlook : charlooks)
			charlook.update(Constants::TIMESTEP);

		if (!beam_done && beam.update())
			beam_done = true;

		if (!sparkle_done && sparkle.update())
			sparkle_done = true;

		if (scroll == Scroll::OPENING && selected >= 0)
		{
			const StatsEntry &stats = characters[selected].stats;
			uint16_t job = stats.stats[MapleStat::Id::JOB];
			bool job_rank = has_rank(stats) && job % 1000 != 0 && job != 2000;

			if (scroll_open[job_rank].update())
				scroll = Scroll::OPEN;
		}

		if (menu_focus >= 0)
			focus_hand[menu_focus].update();
	}

	void UICharSelect::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t slot = slot_at(cursorpos);

		if (slot >= 0 && page * PAGESIZE + slot == selected)
			enter_game();
	}

	Cursor::State UICharSelect::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		int16_t slot = slot_at(cursorpos);

		if (slot >= 0)
		{
			if (clicked)
			{
				int16_t index = page * PAGESIZE + slot;

				if (index != selected)
					select_character(index);

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		if (page_count > 1)
		{
			bool over_arrow = false;

			for (PageArrow *arrow : { &page_left, &page_right })
			{
				Point<int16_t> origin = arrow->position - arrow->normal.get_origin();
				Rectangle<int16_t> bounds(origin, origin + arrow->normal.get_dimensions());
				bool over = bounds.contains(cursorpos);

				if (over && !arrow->hover)
					Sound(Sound::Name::BUTTONOVER).play();

				arrow->hover = over;

				if (!over)
					continue;

				over_arrow = true;

				if (clicked)
				{
					Sound(Sound::Name::BUTTONCLICK).play();
					change_page(arrow == &page_left ? page - 1 : page + 1);
				}
			}

			if (over_arrow)
				return clicked ? Cursor::State::IDLE : Cursor::State::CANCLICK;
		}

		Cursor::State ret = Cursor::State::IDLE;

		for (auto &btit : buttons)
		{
			Button *button = btit.second.get();

			if (button->is_active() && button->bounds(position).contains(cursorpos))
			{
				Button::State state = button->get_state();

				if (state == Button::State::DISABLED)
					continue;

				if (state == Button::State::NORMAL)
				{
					Sound(Sound::Name::BUTTONOVER).play();
					button->set_state(Button::State::MOUSEOVER);
				}

				if (clicked)
				{
					Sound(Sound::Name::BUTTONCLICK).play();
					button->set_state(button_pressed(btit.first));
				} else
				{
					ret = Cursor::State::CANCLICK;
				}
			} else if (button->get_state() == Button::State::MOUSEOVER)
			{
				button->set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	void UICharSelect::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
		{
			button_pressed(Buttons::BT_BACK);
			return;
		}

		switch (keycode)
		{
		case KeyAction::Id::TAB:
			if (menu_focus >= 0)
			{
				menu_focus = -1;
			} else
			{
				menu_focus = Buttons::BT_SELECT;

				if (buttons[menu_focus]->get_state() == Button::State::DISABLED)
					move_focus(1);
			}

			break;
		case KeyAction::Id::UP:
			if (menu_focus >= 0)
				move_focus(-1);

			break;
		case KeyAction::Id::DOWN:
			if (menu_focus >= 0)
				move_focus(1);

			break;
		case KeyAction::Id::LEFT:
			if (menu_focus < 0 && selected > 0)
				select_character(selected - 1);

			break;
		case KeyAction::Id::RIGHT:
			if (menu_focus < 0 && selected + 1 < characters_count)
				select_character(selected + 1);

			break;
		case KeyAction::Id::RETURN:
			if (menu_focus < 0)
				enter_game();
			else if (buttons[menu_focus]->get_state() != Button::State::DISABLED)
				button_pressed(menu_focus);

			break;
		default:
			break;
		}
	}

	UIElement::Type UICharSelect::get_type() const
	{
		return TYPE;
	}

	void UICharSelect::add_character(CharEntry &&character)
	{
		charlooks.emplace_back(character.look);
		names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DUSTYGRAY, character.stats.name);
		characters.emplace_back(std::forward<CharEntry>(character));

		characters_count++;
	}

	void UICharSelect::post_add_character()
	{
		makeactive();
		LoginScene::get().move_to(LoginScene::Step::CHAR_SELECT);

		select_character(characters_count - 1);
		update_buttons();
	}

	void UICharSelect::remove_character(int32_t id)
	{
		for (size_t i = 0; i < characters.size(); i++)
		{
			if (characters[i].id != id)
				continue;

			charlooks.erase(charlooks.begin() + i);
			names.erase(names.begin() + i);
			characters.erase(characters.begin() + i);

			characters_count--;
			selected = -1;
			scroll = Scroll::CLOSED;

			if (characters_count > 0)
				select_character(std::min<int16_t>(i, characters_count - 1));

			update_buttons();

			return;
		}
	}

	const CharEntry &UICharSelect::get_character(int32_t id)
	{
		for (auto &character : characters)
			if (character.id == id)
				return character;

		std::cout << "Invalid character id: [" << id << "]" << std::endl;

		static const CharEntry null_character = {{}, {}, 0};

		return null_character;
	}

	Button::State UICharSelect::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_SELECT:
			enter_game();
			break;
		case Buttons::BT_NEW:
			Sound(Sound::Name::SCROLLUP).play();

			deactivate();
			menu_focus = -1;

			UI::get().emplace<UIRaceSelect>();
			break;
		case Buttons::BT_DELETE:
			delete_character();
			break;
		case Buttons::BT_BACK:
			deactivate();

			Sound(Sound::Name::SCROLLUP).play();

			if (auto worldselect = UI::get().get_element<UIWorldSelect>())
			{
				worldselect->makeactive();
				LoginScene::get().move_to(LoginScene::Step::WORLD_SELECT);
			}

			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	void UICharSelect::update_buttons()
	{
		bool any = characters_count > 0;

		buttons[Buttons::BT_SELECT]->set_state(any ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_DELETE]->set_state(any ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_NEW]->set_state(characters_count < slots ? Button::State::NORMAL : Button::State::DISABLED);

		if (menu_focus >= 0 && buttons[menu_focus]->get_state() == Button::State::DISABLED)
			move_focus(1);
	}

	void UICharSelect::select_character(int16_t index)
	{
		if (index < 0 || index >= characters_count)
			return;

		if (selected >= 0 && selected < characters_count)
			charlooks[selected].set_stance(Stance::Id::STAND1);

		selected = index;
		page = index / PAGESIZE;

		for (size_t i = 0; i < names.size(); i++)
			names[i].change_color(i == static_cast<size_t>(index) ? Color::Name::WHITE : Color::Name::DUSTYGRAY);

		Sound(Sound::Name::CHARSELECT).play();

		charlooks[selected].set_stance(Stance::Id::WALK1);

		beam.reset();
		sparkle.reset();
		beam_done = false;
		sparkle_done = false;

		for (auto &open : scroll_open)
			open.reset();

		scroll = Scroll::OPENING;
	}

	void UICharSelect::change_page(int16_t next)
	{
		if (next < 0)
			next = page_count - 1;
		else if (next >= page_count)
			next = 0;

		page = next;

		int16_t first = page * PAGESIZE;

		// A page with characters picks its first one.
		if (first < characters_count && !(selected >= first && selected < first + PAGESIZE))
			select_character(first);
	}

	void UICharSelect::move_focus(int8_t step)
	{
		for (int8_t i = 0; i < 3; i++)
		{
			menu_focus = (menu_focus + step + 3) % 3;

			if (buttons[menu_focus]->get_state() != Button::State::DISABLED)
			{
				focus_hand[menu_focus].reset();
				return;
			}
		}

		menu_focus = -1;
	}

	int16_t UICharSelect::slot_at(Point<int16_t> cursorpos) const
	{
		for (int16_t i = 0; i < PAGESIZE; i++)
		{
			if (page * PAGESIZE + i >= characters_count)
				break;

			Point<int16_t> feet = position + slot_pos(i);
			Rectangle<int16_t> bounds(
				feet.x() - SLOT_HALF_WIDTH, feet.x() + SLOT_HALF_WIDTH,
				feet.y() - SLOT_HEIGHT, feet.y()
			);

			if (bounds.contains(cursorpos))
				return i;
		}

		return -1;
	}

	void UICharSelect::enter_game()
	{
		if (selected < 0 || selected >= characters_count)
			return;

		Setting<DefaultCharacter>::get().save(selected);
		int32_t id = characters[selected].id;

		switch (require_pic)
		{
		case 0:
		{
			std::function<void()> onok = [&]()
			{
				request_pic();
			};

			UI::get().emplace<UILoginNotice>(UILoginNotice::Message::PIC_REQ, onok);
			break;
		}
		case 1:
		{
			std::function<void(const std::string &)> onok = [id](const std::string &pic)
			{
				SelectCharPicPacket(pic, id).dispatch();
			};

			UI::get().emplace<UISoftKey>(onok);
			break;
		}
		case 2:
			SelectCharPacket(id).dispatch();
			break;
		}
	}

	void UICharSelect::delete_character()
	{
		if (selected < 0 || selected >= characters_count)
			return;

		int32_t id = characters[selected].id;

		switch (require_pic)
		{
		case 0:
			UI::get().emplace<UILoginNotice>(UILoginNotice::Message::CHAR_DEL_FAIL_NO_PIC);
			break;
		case 1:
		{
			std::function<void()> onok = [id]()
			{
				std::function<void(const std::string &)> onpic = [id](const std::string &pic)
				{
					DeleteCharPicPacket(pic, id).dispatch();
				};

				UI::get().emplace<UISoftKey>(onpic);
			};

			uint16_t job = characters[selected].stats.stats[MapleStat::Id::JOB];

			if (job < 1000)
				UI::get().emplace<UILoginNotice>(UILoginNotice::Message::DELETE_CONFIRMATION, onok);
			else
				UI::get().emplace<UILoginNotice>(UILoginNotice::Message::CASH_ITEMS_CONFIRM_DELETION, onok);

			break;
		}
		case 2:
			DeleteCharPacket(id).dispatch();
			break;
		}
	}

	void UICharSelect::request_pic()
	{
		std::function<void(const std::string &)> enterpic = [&](const std::string &entered_pic)
		{
			std::function<void(const std::string &)> verifypic = [&, entered_pic](const std::string &verify_pic)
			{
				if (entered_pic == verify_pic)
				{
					check_pic(entered_pic);
				} else
				{
					std::function<void()> onreturn = [&]()
					{
						request_pic();
					};

					UI::get().emplace<UILoginNotice>(UILoginNotice::Message::PASSWORD_IS_INCORRECT, onreturn);
				}
			};

			UI::get().emplace<UISoftKey>(verifypic, []()
			{}, "Please re-enter your new PIC.", Point<int16_t>(24, 0));
		};

		UI::get().emplace<UISoftKey>(enterpic, []()
		{}, "Your new PIC must at least be 6 characters long.", Point<int16_t>(24, 0));
	}

	void UICharSelect::check_pic(const std::string entered_pic) const
	{
		const char *pStr = entered_pic.c_str();

		if (pStr == NULL)
			return;

		int count = 0;
		char m = ' ';
		bool reptitive = false;

		while (*pStr)
		{
			if (*pStr == m)
			{
				count++;
			} else
			{
				count = 0;
				m = *pStr;
			}

			if (count > 2)
			{
				reptitive = true;
				break;
			}

			pStr++;
		}

		if (reptitive)
		{
			UI::get().emplace<UILoginNotice>(UILoginNotice::Message::PIC_REPITIVE);
		} else
		{
			RegisterPicPacket(
					characters[selected].id,
					entered_pic
			).dispatch();
		}
	}
}
